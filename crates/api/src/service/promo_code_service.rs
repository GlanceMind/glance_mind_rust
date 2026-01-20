use crate::config::database::Database;
use crate::dto::promo_code_dto::{RedeemPromoCodeRequest, RedeemPromoCodeResponse};
use glance_mind_db::entity::promo_code::PromoCode;
use crate::error::api_error::ApiError;
use crate::repository::wallet_repository::WalletRepository;
use glance_mind_db::schema::gm_promo_codes;
use bigdecimal::BigDecimal;
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub struct PromoCodeService {
    db: Arc<Database>,
    wallet_repo: WalletRepository,
}

impl PromoCodeService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            db: db.clone(),
            wallet_repo: WalletRepository::new(db.pool.clone()),
        }
    }

    /// Redeem promo code
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

        // Query promo code
        let promo_code = gm_promo_codes::table
            .filter(gm_promo_codes::code.eq(&code))
            .first::<PromoCode>(&mut conn)
            .optional()
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to query promo code: {}", e))
            })?;

        let promo_code = match promo_code {
            Some(p) => p,
            None => {
                return Ok(RedeemPromoCodeResponse {
                    success: false,
                    message: "Promo code not found".to_string(),
                    points: None,
                    new_balance: None,
                });
            }
        };

        // Check if already used
        if promo_code.used_by_user_id.is_some() {
            return Ok(RedeemPromoCodeResponse {
                success: false,
                message: "Promo code has already been used".to_string(),
                points: None,
                new_balance: None,
            });
        }

        // Check if active
        if !promo_code.is_active {
            return Ok(RedeemPromoCodeResponse {
                success: false,
                message: "Promo code is not active".to_string(),
                points: None,
                new_balance: None,
            });
        }

        // Check if expired
        if let Some(expires_at) = promo_code.expires_at {
            if expires_at < Utc::now() {
                return Ok(RedeemPromoCodeResponse {
                    success: false,
                    message: "Promo code has expired".to_string(),
                    points: None,
                    new_balance: None,
                });
            }
        }

        // Start redemption process (transaction)
        let points_to_add = promo_code.points;
        let promo_code_id = promo_code.id;

        // Use database transaction to ensure atomicity
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            // 1. Mark promo code as used
            diesel::update(gm_promo_codes::table)
                .filter(gm_promo_codes::id.eq(promo_code_id))
                .set((
                    gm_promo_codes::used_by_user_id.eq(Some(user_id)),
                    gm_promo_codes::used_at.eq(Some(Utc::now())),
                    gm_promo_codes::updated_at.eq(Some(Utc::now())),
                ))
                .execute(conn)?;

            Ok(())
        })
        .map_err(|e| {
            ApiError::InternalServerError(format!("Failed to mark promo code as used: {}", e))
        })?;

        // 2. Add user balance and create transaction record (in separate transaction)
        let amount = BigDecimal::from(points_to_add);
        let (wallet, _transaction) = self
            .wallet_repo
            .add_balance_with_transaction(
                user_id,
                amount,
                "PROMO_CODE".to_string(),
                format!("Promo code redeemed: {}", code),
            )
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to add balance: {}", e)))?;

        // Calculate new balance (available balance = total balance - frozen balance)
        let new_balance = (&wallet.balance_points - &wallet.frozen_points)
            .to_string()
            .parse::<f64>()
            .unwrap_or(0.0) as i32;

        Ok(RedeemPromoCodeResponse {
            success: true,
            message: format!("Successfully redeemed {} points!", points_to_add),
            points: Some(points_to_add),
            new_balance: Some(new_balance),
        })
    }
}
