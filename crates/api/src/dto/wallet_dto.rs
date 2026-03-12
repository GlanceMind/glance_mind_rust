use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentStatus {
    Pending,
    Paid,
    Failed,
    Refunding,
    Refunded,
}

impl PaymentStatus {
    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Paid => "PAID",
            Self::Failed => "FAILED",
            Self::Refunding => "REFUNDING",
            Self::Refunded => "REFUNDED",
        }
    }

    pub fn from_db_value(value: Option<&str>) -> Option<Self> {
        match value {
            Some("PENDING") => Some(Self::Pending),
            Some("PAID") => Some(Self::Paid),
            Some("FAILED") => Some(Self::Failed),
            Some("REFUNDING") => Some(Self::Refunding),
            Some("REFUNDED") => Some(Self::Refunded),
            _ => None,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Failed | Self::Refunded)
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        match self {
            Self::Pending => matches!(
                next,
                Self::Paid | Self::Failed | Self::Refunding | Self::Refunded
            ),
            Self::Paid => matches!(next, Self::Refunding | Self::Refunded),
            Self::Refunding => matches!(next, Self::Paid | Self::Failed | Self::Refunded),
            Self::Failed | Self::Refunded => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RechargeChannel {
    Wechat,
    Alipay,
}

impl RechargeChannel {
    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::Wechat => "wechat",
            Self::Alipay => "alipay",
        }
    }

    pub fn from_db_value(value: Option<&str>) -> Option<Self> {
        match value {
            Some("wechat") => Some(Self::Wechat),
            Some("alipay") => Some(Self::Alipay),
            _ => None,
        }
    }
}

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
    pub payment_status: Option<PaymentStatus>,
    pub created_at: DateTime<Utc>,
}

// --------------- XunhuPay recharge DTOs ---------------

#[derive(Debug, Deserialize)]
pub struct RechargeRequestDto {
    /// Amount in CNY (e.g. "9.90")
    pub amount: String,
    /// Supported channels: wechat / alipay
    pub channel: RechargeChannel,
    /// Optional: URL to redirect after payment
    pub return_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RechargeResponseDto {
    pub order_no: String,
    pub payment_url: Option<String>,
    pub qrcode_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RechargeOrderStatusDto {
    pub order_no: String,
    pub status: PaymentStatus,
    pub amount: String,
    pub channel: RechargeChannel,
    pub paid_at: Option<DateTime<Utc>>,
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
    pub total_spent: BigDecimal,        // 总花费 (actual_consumption sum)
    pub active_campaigns: i64,          // 活跃任务数
    pub total_campaigns: i64,           // 总任务数
    pub interaction_scanned_count: i64, // 本月互动数 (total_scanned)
    pub total_replied_count: i64,       // 总回复数
}

#[derive(Debug, Serialize)]
pub struct PerformanceDataDto {
    pub date: String,
    pub spent: BigDecimal,
    pub reach: i32,
    pub conversions: i32,
}

// Recent Campaign Activity DTO
#[derive(Debug, Serialize)]
pub struct RecentCampaignDto {
    pub id: i32,
    pub name: String,
    pub status: String,
    pub platform_name: String,
    pub actual_consumption: BigDecimal,
    pub total_scanned: i32,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

// Password Change DTO
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ChangePasswordDto {
    pub old_password: String,
    pub new_password: String,
}
