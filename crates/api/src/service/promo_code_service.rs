use crate::config::database::Database;
use crate::dto::promo_code_dto::{RedeemPromoCodeRequest, RedeemPromoCodeResponse};
use crate::error::api_error::ApiError;
use bigdecimal::BigDecimal;
use chrono::Utc;
use diesel::prelude::*;
use glance_mind_db::entity::promo_code::PromoCode;
use glance_mind_db::schema::gm_promo_codes;
use std::sync::Arc;

#[derive(Clone)]
pub struct PromoCodeService {
    db: Arc<Database>,
}

impl PromoCodeService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self { db: db.clone() }
    }

    /// Redeem promo code with concurrent safety
    ///
    /// This function uses a single atomic transaction with row-level locking
    /// to prevent race conditions where the same promo code could be redeemed
    /// multiple times by concurrent requests.
    pub async fn redeem_promo_code(
        &self,
        user_id: i32,
        request: RedeemPromoCodeRequest,
    ) -> Result<RedeemPromoCodeResponse, ApiError> {
        let code = request.code.trim().to_lowercase();

        // Validate promo code format (simple length check)
        if code.is_empty() || code.len() < 10 {
            return Ok(RedeemPromoCodeResponse {
                success: false,
                message: "Invalid promo code format".to_string(),
                points: None,
                new_balance: None,
            });
        }

        let mut conn =
            self.db.pool.get().map_err(|_| {
                ApiError::InternalServerError("Database connection failed".to_string())
            })?;

        // Execute all operations in a single atomic transaction
        let result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
            // 1. Lock the promo code row with FOR UPDATE to prevent concurrent redemptions
            // This ensures only one transaction can proceed at a time for this promo code
            let promo_code = gm_promo_codes::table
                .filter(gm_promo_codes::code.eq(&code))
                .for_update() // 🔒 Critical: Row-level lock
                .first::<PromoCode>(conn)
                .optional()?;

            let promo_code = match promo_code {
                Some(p) => p,
                None => {
                    // Promo code not found - return custom error to distinguish from validation errors
                    return Err(diesel::result::Error::NotFound);
                }
            };

            // 2. Re-validate all conditions INSIDE the transaction after acquiring lock
            // This prevents TOCTOU (Time-Of-Check-Time-Of-Use) race conditions

            // Check if already used (critical check inside transaction)
            if promo_code.used_by_user_id.is_some() {
                return Err(diesel::result::Error::RollbackTransaction);
            }

            // Check if active
            if !promo_code.is_active {
                // Use a distinct error variant for inactive codes
                return Err(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::CheckViolation,
                    Box::new("INACTIVE".to_string()),
                ));
            }

            // Check if expired
            if let Some(expires_at) = promo_code.expires_at {
                if expires_at < Utc::now() {
                    // Use a distinct error variant for expired codes
                    return Err(diesel::result::Error::DatabaseError(
                        diesel::result::DatabaseErrorKind::UniqueViolation,
                        Box::new("EXPIRED".to_string()),
                    ));
                }
            }

            let points_to_add = promo_code.points;
            let amount = BigDecimal::from(points_to_add);

            // 3. Mark promo code as used (still in the same transaction)
            diesel::update(gm_promo_codes::table)
                .filter(gm_promo_codes::id.eq(promo_code.id))
                .set((
                    gm_promo_codes::used_by_user_id.eq(Some(user_id)),
                    gm_promo_codes::used_at.eq(Some(Utc::now())),
                    gm_promo_codes::updated_at.eq(Some(Utc::now())),
                ))
                .execute(conn)?;

            // 4. Ensure wallet exists, create if not (still in the same transaction)
            use glance_mind_db::schema::gm_user_wallets as user_wallets;

            let wallet_exists = user_wallets::table
                .filter(user_wallets::user_id.eq(user_id))
                .select(user_wallets::user_id)
                .first::<i32>(conn)
                .optional()?;

            if wallet_exists.is_none() {
                diesel::insert_into(user_wallets::table)
                    .values((
                        user_wallets::user_id.eq(user_id),
                        user_wallets::balance_points.eq(BigDecimal::from(0)),
                        user_wallets::frozen_points.eq(BigDecimal::from(0)),
                    ))
                    .execute(conn)?;
            }

            // 5. Update wallet balance (still in the same transaction)
            use glance_mind_db::entity::user_wallet::UserWallet;
            let wallet = diesel::update(user_wallets::table)
                .filter(user_wallets::user_id.eq(user_id))
                .set(user_wallets::balance_points.eq(user_wallets::balance_points + &amount))
                .get_result::<UserWallet>(conn)?;

            // 6. Create transaction record (still in the same transaction)
            use glance_mind_db::entity::wallet_transaction::NewWalletTransaction;
            use glance_mind_db::schema::gm_wallet_transactions as wallet_transactions;

            let new_transaction = NewWalletTransaction {
                user_id,
                amount: amount.clone(),
                type_: "PROMO_CODE".to_string(),
                payment_method: None,
                external_txn_id: None,
                reference_id: Some(promo_code.id),
                description: Some(format!("Promo code redeemed: {}", code)),
                reference_type: Some("promo_code".to_string()),
                payment_status: None,
            };

            diesel::insert_into(wallet_transactions::table)
                .values(&new_transaction)
                .execute(conn)?;

            // Calculate new balance
            let new_balance = (&wallet.balance_points - &wallet.frozen_points)
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0) as i32;

            Ok((points_to_add, new_balance))
        });

        // Handle transaction result and map to appropriate response
        match result {
            Ok((points, new_balance)) => {
                tracing::info!(
                    "Promo code '{}' successfully redeemed by user {} for {} points",
                    code,
                    user_id,
                    points
                );
                Ok(RedeemPromoCodeResponse {
                    success: true,
                    message: format!("Successfully redeemed {} points!", points),
                    points: Some(points),
                    new_balance: Some(new_balance),
                })
            }
            Err(diesel::result::Error::NotFound) => Ok(RedeemPromoCodeResponse {
                success: false,
                message: "Promo code not found".to_string(),
                points: None,
                new_balance: None,
            }),
            Err(diesel::result::Error::RollbackTransaction) => Ok(RedeemPromoCodeResponse {
                success: false,
                message: "Promo code has already been used".to_string(),
                points: None,
                new_balance: None,
            }),
            Err(diesel::result::Error::DatabaseError(kind, _)) => {
                let message = match kind {
                    diesel::result::DatabaseErrorKind::Unknown => {
                        // These are our custom validation errors
                        "Promo code validation failed".to_string()
                    }
                    _ => "Database error occurred".to_string(),
                };
                tracing::warn!("Promo code redemption error: {:?}", kind);
                Ok(RedeemPromoCodeResponse {
                    success: false,
                    message,
                    points: None,
                    new_balance: None,
                })
            }
            Err(e) => {
                tracing::error!("Promo code redemption failed: {:?}", e);
                Err(ApiError::InternalServerError(format!(
                    "Failed to redeem promo code: {}",
                    e
                )))
            }
        }
    }
}
