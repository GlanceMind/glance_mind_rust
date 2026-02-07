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
    /// Business type: campaign, aipub_plan, etc.
    pub reference_type: Option<String>,
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
    /// Business type: campaign, aipub_plan, etc.
    pub reference_type: Option<String>,
}
