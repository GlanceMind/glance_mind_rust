use super::types::ActionType;
use crate::error::api_error::ApiError;
use crate::repository::ai_model_repository::AiModelRepository;
use crate::repository::pricing_repository::PricingRepository;
use crate::repository::wallet_repository::WalletRepository;
use bigdecimal::BigDecimal;
use glance_mind_db::entity::wallet_transaction::WalletTransaction;

#[derive(Clone)]
pub struct ChargingManager {
    pricing_repo: PricingRepository,
    ai_model_repo: AiModelRepository,
    wallet_repo: WalletRepository,
}

#[derive(Clone, Debug)]
pub struct ChargingContext {
    pub action_type: ActionType,
    pub ai_model_id: Option<i32>,
    pub platform_id: Option<i32>,
    pub base_cost: BigDecimal,
    pub cost_multiplier: BigDecimal,
    pub final_cost: BigDecimal,
}

fn calculate_final_cost(base_cost: &BigDecimal, cost_multiplier: &BigDecimal) -> BigDecimal {
    base_cost * cost_multiplier
}

fn points_to_rmb(points: &BigDecimal) -> BigDecimal {
    points / BigDecimal::from(100)
}

impl ChargingManager {
    pub fn new(
        pricing_repo: PricingRepository,
        ai_model_repo: AiModelRepository,
        wallet_repo: WalletRepository,
    ) -> Self {
        Self {
            pricing_repo,
            ai_model_repo,
            wallet_repo,
        }
    }

    /// Step 1: Calculate cost and check balance
    pub async fn prepare_charging(
        &self,
        user_id: i32,
        action_type: ActionType,
        ai_model_id: Option<i32>,
        platform_id: Option<i32>,
    ) -> Result<ChargingContext, ApiError> {
        // 1. Query pricing_rule - use different query logic based on action_type
        let pricing_rule = self
            .pricing_repo
            .find_by_action_type(action_type.as_str(), platform_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to query pricing rule: {:?}", e);
                ApiError::InternalServerError("Failed to query pricing rule".to_string())
            })?
            .ok_or_else(|| {
                let error_msg = match platform_id {
                    Some(pid) => format!(
                        "Pricing rule not found: {} (platform_id: {})",
                        action_type.as_str(),
                        pid
                    ),
                    None => format!("Pricing rule not found: {}", action_type.as_str()),
                };
                ApiError::PricingRuleNotFound(error_msg)
            })?;

        let base_cost = pricing_rule.cost_points;

        // 2. Query ai_model multiplier (if ai_model_id provided)
        let cost_multiplier = if let Some(model_id) = ai_model_id {
            let model = self
                .ai_model_repo
                .find_by_id(model_id)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to query AI model: {:?}", e);
                    ApiError::InternalServerError("Failed to query AI model".to_string())
                })?
                .ok_or_else(|| ApiError::BadRequest(format!("AI model not found: {}", model_id)))?;

            if !model.is_active {
                return Err(ApiError::BadRequest("AI model not active".to_string()));
            }

            model.cost_multiplier
        } else {
            BigDecimal::from(1)
        };

        // 3. Calculate final cost: base_cost * cost_multiplier
        let final_cost = calculate_final_cost(&base_cost, &cost_multiplier);
        let final_cost_rmb = points_to_rmb(&final_cost);

        tracing::info!(
            "Charging preparation: user_id={}, action={:?}, platform_id={:?}, base_cost={}, multiplier={}, final_cost={} (~¥{})",
            user_id,
            action_type,
            platform_id,
            base_cost,
            cost_multiplier,
            final_cost,
            final_cost_rmb
        );

        // 4. Check balance
        let has_balance = self
            .wallet_repo
            .validate_available_balance(user_id, &final_cost)
            .await
            .map_err(|e| {
                tracing::error!("Failed to check balance: {:?}", e);
                ApiError::InternalServerError("Failed to check balance".to_string())
            })?;

        if !has_balance {
            tracing::warn!(
                "User {} insufficient balance, needs {} points",
                user_id,
                final_cost
            );
            return Err(ApiError::InsufficientBalance);
        }

        // 5. Return charging context
        Ok(ChargingContext {
            action_type,
            ai_model_id,
            platform_id,
            base_cost,
            cost_multiplier,
            final_cost,
        })
    }

    /// Step 2: Execute charge and record transaction
    pub async fn execute_charging(
        &self,
        user_id: i32,
        context: &ChargingContext,
        reference_id: Option<String>,
    ) -> Result<WalletTransaction, ApiError> {
        let action_name = match context.action_type {
            ActionType::AiAnalyze => "AI Analysis",
            ActionType::VideoGenerate => "Video Generation",
            ActionType::ScanPost => "Scan Post",
        };

        let mut description = format!(
            "{} - consumed {} points (base: {}, multiplier: {})",
            action_name, context.final_cost, context.base_cost, context.cost_multiplier
        );

        // If platform_id exists, add to description
        if let Some(pid) = context.platform_id {
            description.push_str(&format!(" [platform_id: {}]", pid));
        }

        // If reference_id exists, add to description
        if let Some(ref_id) = reference_id {
            description.push_str(&format!(" [ref: {}]", ref_id));
        }

        tracing::info!(
            "Executing charge: user_id={}, amount={}, description={}",
            user_id,
            context.final_cost,
            description
        );

        // Deduct (pass negative value)
        let (_, transaction) = self
            .wallet_repo
            .add_balance_with_transaction(
                user_id,
                -context.final_cost.clone(),
                "DEBIT".to_string(),
                description,
            )
            .await
            .map_err(|e| {
                tracing::error!("Charge failed: {:?}", e);
                ApiError::ChargeFailed(format!("Charge failed: {:?}", e))
            })?;

        tracing::info!("Charge successful: transaction_id={}", transaction.id);

        Ok(transaction)
    }
}

#[cfg(test)]
mod tests {
    use super::{calculate_final_cost, points_to_rmb};
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    fn bd(value: &str) -> BigDecimal {
        BigDecimal::from_str(value).expect("valid decimal")
    }

    #[test]
    fn calculate_final_cost_is_table_driven() {
        let cases = [
            ("1.00", "1.00", "1.00"),
            ("2.00", "1.50", "3.00"),
            ("400.00", "1.00", "400.00"),
            ("400.00", "1.50", "600.00"),
            ("400.00", "3.00", "1200.00"),
            ("400.00", "3.75", "1500.00"),
        ];

        for (base_cost, multiplier, expected) in cases {
            let actual = calculate_final_cost(&bd(base_cost), &bd(multiplier));
            assert_eq!(
                actual.normalized(),
                bd(expected).normalized(),
                "base_cost={base_cost}, multiplier={multiplier}"
            );
        }
    }

    #[test]
    fn points_to_rmb_uses_points_first_exchange_rate() {
        let cases = [
            ("0.00", "0.00"),
            ("3.00", "0.03"),
            ("10.00", "0.10"),
            ("400.00", "4.00"),
            ("1200.00", "12.00"),
            ("1500.00", "15.00"),
        ];

        for (points, expected_rmb) in cases {
            let actual = points_to_rmb(&bd(points));
            assert_eq!(
                actual.normalized(),
                bd(expected_rmb).normalized(),
                "points={points}"
            );
        }
    }
}
