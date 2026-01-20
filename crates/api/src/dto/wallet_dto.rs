use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

// Wallet DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct WalletBalanceDto {
    pub user_id: i32,
    pub balance_points: BigDecimal,
    pub frozen_points: BigDecimal,
    pub available_balance: BigDecimal,
    pub deposit_cny: BigDecimal,
    pub deposit_usd: BigDecimal,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletTransactionDto {
    pub id: i32,
    pub user_id: i32,
    pub amount: BigDecimal,
    #[serde(rename = "type")]
    pub type_: String,
    pub payment_method: Option<String>,
    pub external_txn_id: Option<String>,
    pub reference_id: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopUpRequestDto {
    pub amount: BigDecimal,
    pub payment_method: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopUpResponseDto {
    pub transaction_id: i32,
    pub payment_url: Option<String>, // Mocking a payment gateway URL
    pub message: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionHistoryQueryDto {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub type_: Option<String>,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

// Dashboard DTOs
#[derive(Debug, Serialize)]
pub struct OverviewStatsDto {
    pub total_budget: BigDecimal,
    pub active_campaigns: i64,
    pub interaction_scanned_count: i64,
    pub relevant_comments_count: i64,
    pub replied_count: i64,
}

#[derive(Debug, Serialize)]
pub struct PerformanceDataDto {
    pub date: String,
    pub spent: BigDecimal,
    pub reach: i32,
    pub conversions: i32,
}

// Password Change DTO
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ChangePasswordDto {
    pub old_password: String,
    pub new_password: String,
}
