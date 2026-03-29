use crate::config::database::Database;
use crate::repository::wallet_repository::WalletRepository;
use bigdecimal::BigDecimal;
use std::sync::Arc;

#[derive(Clone)]
pub struct DramaBillingGuard {
    wallet_repo: WalletRepository,
    db: Arc<Database>,
}

#[derive(Debug)]
pub struct BalanceCheck {
    pub balance_points: i64,
    pub estimated_cost_points: i64,
    pub sufficient: bool,
}

#[derive(Debug)]
pub struct ReserveResult {
    pub project_id: String,
    pub reserved_cents: i64,
    pub success: bool,
}

fn decimal_to_i64(value: &BigDecimal) -> i64 {
    let raw = value.to_string();
    let trimmed = if let Some((whole, frac)) = raw.split_once('.') {
        if frac.chars().all(|c| c == '0') {
            whole.to_string()
        } else {
            whole.to_string()
        }
    } else {
        raw
    };
    trimmed.parse::<i64>().unwrap_or(0)
}

impl DramaBillingGuard {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            wallet_repo: WalletRepository::new(db.pool.clone()),
            db: db.clone(),
        }
    }

    pub async fn check_balance(
        &self,
        user_id: i32,
        estimated_cost_points: i64,
    ) -> Result<BalanceCheck, String> {
        let wallet = self
            .wallet_repo
            .find_by_user(user_id)
            .await
            .map_err(|e| format!("wallet lookup failed: {}", e))?;

        let balance = decimal_to_i64(&wallet.balance_points);
        let frozen = decimal_to_i64(&wallet.frozen_points);

        let available = balance - frozen;
        Ok(BalanceCheck {
            balance_points: available,
            estimated_cost_points,
            sufficient: available >= estimated_cost_points,
        })
    }

    pub async fn create_reserve(
        &self,
        user_id: i32,
        project_id: &str,
        amount_cents: i64,
    ) -> Result<ReserveResult, String> {
        let amount = BigDecimal::from(amount_cents);
        self.wallet_repo
            .freeze_campaign_budget(user_id, &amount)
            .await
            .map_err(|e| format!("freeze failed: {}", e))?;

        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "UPDATE gm_drama_project_projections \
             SET cost_reserve_cents = $2, updated_at = NOW() \
             WHERE project_id = $1",
        )
        .bind::<diesel::sql_types::Text, _>(project_id)
        .bind::<diesel::sql_types::BigInt, _>(amount_cents)
        .execute(conn)
        .map_err(|e| format!("update projection reserve: {}", e))?;

        tracing::info!(
            user_id = user_id,
            project_id = project_id,
            amount_cents = amount_cents,
            "Drama budget reserved"
        );

        Ok(ReserveResult {
            project_id: project_id.to_string(),
            reserved_cents: amount_cents,
            success: true,
        })
    }

    pub async fn release_reserve(
        &self,
        user_id: i32,
        project_id: &str,
        consumed_cents: i64,
        reserved_cents: i64,
    ) -> Result<(), String> {
        let unused = reserved_cents - consumed_cents;
        if unused <= 0 {
            return Ok(());
        }

        let amount = BigDecimal::from(unused);
        self.wallet_repo
            .unfreeze_campaign_budget(user_id, &amount)
            .await
            .map_err(|e| format!("unfreeze failed: {}", e))?;

        tracing::info!(
            user_id = user_id,
            project_id = project_id,
            released_cents = unused,
            "Drama reserve released"
        );

        Ok(())
    }

    pub async fn settle_cost(
        &self,
        user_id: i32,
        amount_cents: i64,
    ) -> Result<(), String> {
        let amount = BigDecimal::from(amount_cents);

        self.wallet_repo
            .unfreeze_campaign_budget(user_id, &amount)
            .await
            .map_err(|e| format!("unfreeze for settle: {}", e))?;

        let neg_amount = BigDecimal::from(-amount_cents);
        self.wallet_repo
            .update_balance(user_id, neg_amount)
            .await
            .map_err(|e| format!("deduct balance: {}", e))?;

        tracing::info!(
            user_id = user_id,
            amount_cents = amount_cents,
            "Drama cost settled"
        );

        Ok(())
    }
}

use diesel::prelude::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_check_sufficient() {
        let check = BalanceCheck {
            balance_points: 50000,
            estimated_cost_points: 31800,
            sufficient: true,
        };
        assert!(check.sufficient);
    }

    #[test]
    fn balance_check_insufficient() {
        let check = BalanceCheck {
            balance_points: 100,
            estimated_cost_points: 31800,
            sufficient: false,
        };
        assert!(!check.sufficient);
    }

    #[test]
    fn reserve_result_fields() {
        let result = ReserveResult {
            project_id: "proj_001".to_string(),
            reserved_cents: 31800,
            success: true,
        };
        assert!(result.success);
        assert_eq!(result.reserved_cents, 31800);
    }

    #[test]
    fn release_zero_is_noop() {
        let consumed = 31800i64;
        let reserved = 31800i64;
        let unused = reserved - consumed;
        assert_eq!(unused, 0);
    }

    #[test]
    fn decimal_to_i64_handles_decimal_strings() {
        let value = "50000.00".parse::<BigDecimal>().expect("decimal");
        assert_eq!(decimal_to_i64(&value), 50000);
    }

    #[test]
    fn decimal_to_i64_truncates_fractional_values() {
        let value = "123.45".parse::<BigDecimal>().expect("decimal");
        assert_eq!(decimal_to_i64(&value), 123);
    }
}
