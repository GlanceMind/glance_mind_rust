use crate::schema::gm_wallet_transactions;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Insertable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_wallet_transactions)]
pub struct WalletTransaction {
    pub id: i32,
    pub user_id: i32,
    pub amount: BigDecimal,
    #[serde(rename = "type")]
    pub type_: String,
    pub payment_method: Option<String>,
    pub external_txn_id: Option<String>,
    pub reference_id: Option<i32>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    /// Business type: campaign, aipub_plan, promo_code, etc.
    pub reference_type: Option<String>,
    /// Payment lifecycle: PENDING, PAID, FAILED, REFUNDING, REFUNDED
    pub payment_status: Option<String>,
    /// Third-party platform transaction id
    pub platform_txn_id: Option<String>,
    /// XunhuPay internal order id
    pub open_order_id: Option<String>,
    /// Actual payment completion time
    pub paid_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Serialize, Deserialize)]
#[diesel(table_name = gm_wallet_transactions)]
pub struct NewWalletTransaction {
    pub user_id: i32,
    pub amount: BigDecimal,
    #[serde(rename = "type")]
    pub type_: String,
    pub payment_method: Option<String>,
    pub external_txn_id: Option<String>,
    pub reference_id: Option<i32>,
    pub description: Option<String>,
    /// Business type: campaign, aipub_plan, promo_code, etc.
    pub reference_type: Option<String>,
    /// Payment lifecycle: PENDING, PAID, FAILED, REFUNDING, REFUNDED
    pub payment_status: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_billing_harness_recharge_contract_fields() {
        let recharge = NewWalletTransaction {
            user_id: 42,
            amount: BigDecimal::from(2000),
            type_: "RECHARGE".to_string(),
            payment_method: Some("alipay".to_string()),
            external_txn_id: Some("order-123".to_string()),
            reference_id: None,
            description: Some("Harness recharge".to_string()),
            reference_type: Some("recharge".to_string()),
            payment_status: Some("PENDING".to_string()),
        };

        assert_eq!(recharge.user_id, 42);
        assert_eq!(recharge.type_, "RECHARGE");
        assert_eq!(recharge.payment_method.as_deref(), Some("alipay"));
        assert_eq!(recharge.external_txn_id.as_deref(), Some("order-123"));
        assert_eq!(recharge.payment_status.as_deref(), Some("PENDING"));
    }

    #[test]
    fn wallet_billing_harness_deposit_contract_fields() {
        let deposit = NewWalletTransaction {
            user_id: 42,
            amount: BigDecimal::from(2000),
            type_: "DEPOSIT".to_string(),
            payment_method: Some("alipay".to_string()),
            external_txn_id: Some("order-123".to_string()),
            reference_id: Some(7),
            description: Some("Harness deposit".to_string()),
            reference_type: Some("recharge".to_string()),
            payment_status: Some("PAID".to_string()),
        };

        assert_eq!(deposit.type_, "DEPOSIT");
        assert_eq!(deposit.reference_type.as_deref(), Some("recharge"));
        assert_eq!(deposit.reference_id, Some(7));
        assert_eq!(deposit.payment_status.as_deref(), Some("PAID"));
    }
}
