use crate::config::database::Database;
use crate::dto::wallet_dto::{
    PaymentStatus, RechargeChannel, RechargeOrderStatusDto, RechargeRequestDto,
    RechargeResponseDto, WalletBalanceDto, WalletTransactionDto,
};
use crate::error::api_error::ApiError;
use crate::repository::wallet_repository::{RefundApplyOutcome, WalletRepository};
use crate::service::xunhupay_client::{
    OrderId, PayNotification, PayRequest, QueryResponse, XunhuPayClient,
};
use bigdecimal::BigDecimal;
use chrono::{Duration as ChronoDuration, Utc};
use glance_mind_db::entity::wallet_transaction::{NewWalletTransaction, WalletTransaction};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

const MAX_RECHARGE_CNY: i64 = 10_000;
// Set to 0 to temporarily disable the pending-order creation limit.
const MAX_PENDING_RECHARGE_ORDERS: i64 = 0;

#[derive(Clone)]
pub struct WalletService {
    repo: WalletRepository,
    xunhupay: Option<XunhuPayClient>,
    notify_base_url: Option<String>,
    reconciliation_interval_secs: u64,
    reconciliation_pending_age_secs: i64,
    reconciliation_batch_size: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum XunhuPayOrderStatus {
    Pending,
    Paid,
    Refunding,
    Cancelled,
    Refunded,
    RefundFailed,
    Unknown,
}

impl XunhuPayOrderStatus {
    fn from_notify(raw_status: &str) -> Self {
        match raw_status {
            "OD" => Self::Paid,
            "RD" => Self::Refunding,
            "CD" => Self::Refunded,
            "UD" => Self::RefundFailed,
            _ => Self::Unknown,
        }
    }

    fn from_query(raw_status: Option<&str>) -> Self {
        match raw_status {
            Some("WP") | None => Self::Pending,
            Some("OD") => Self::Paid,
            Some("RD") => Self::Refunding,
            Some("CD") => Self::Cancelled,
            Some("UD") => Self::RefundFailed,
            Some(_) => Self::Unknown,
        }
    }
}

impl WalletService {
    pub fn new(db: &Arc<Database>) -> Self {
        let xunhupay = Self::init_xunhupay();
        let notify_base_url = std::env::var("API_BASE_URL")
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty());
        let reconciliation_interval_secs = std::env::var("RECHARGE_RECONCILIATION_INTERVAL_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(300);
        let reconciliation_pending_age_secs =
            std::env::var("RECHARGE_RECONCILIATION_PENDING_AGE_SECS")
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(15);
        let reconciliation_batch_size = std::env::var("RECHARGE_RECONCILIATION_BATCH_SIZE")
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(20);

        Self {
            repo: WalletRepository::new(db.pool.clone()),
            xunhupay,
            notify_base_url,
            reconciliation_interval_secs,
            reconciliation_pending_age_secs,
            reconciliation_batch_size,
        }
    }

    fn init_xunhupay() -> Option<XunhuPayClient> {
        let app_id = std::env::var("XUNHUPAY_APP_ID").ok()?;
        let app_secret = std::env::var("XUNHUPAY_APP_SECRET").ok()?;
        if app_id.is_empty() || app_secret.is_empty() {
            return None;
        }
        let gateway = std::env::var("XUNHUPAY_GATEWAY_URL").ok();
        tracing::info!("XunhuPay client initialized (appid={})", app_id);
        Some(XunhuPayClient::new(app_id, app_secret, gateway))
    }

    pub fn spawn_recharge_reconciliation_loop(&self) {
        if self.xunhupay.is_none() || self.reconciliation_interval_secs == 0 {
            tracing::info!("Recharge reconciliation loop disabled");
            return;
        }

        let wallet_service = self.clone();
        let interval_secs = self.reconciliation_interval_secs;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            interval.tick().await;
            loop {
                interval.tick().await;
                match wallet_service.reconcile_stale_pending_orders().await {
                    Ok(reconciled) if reconciled > 0 => {
                        tracing::info!(
                            "Recharge reconciliation loop finalized {reconciled} order(s)"
                        );
                    }
                    Ok(_) => {}
                    Err(error) => {
                        tracing::warn!("Recharge reconciliation loop failed: {error:?}");
                    }
                }
            }
        });
    }

    pub async fn get_balance(&self, user_id: i32) -> Result<WalletBalanceDto, ApiError> {
        // Try to find existing wallet, or create new one
        let wallet = match self.repo.find_by_user(user_id).await {
            Ok(w) => w,
            Err(_) => {
                // Create new wallet with zero balance
                self.repo.create(user_id).await.map_err(|_| {
                    ApiError::InternalServerError("Failed to create wallet".to_string())
                })?
            }
        };
        // Calculate available balance
        let available_balance = &wallet.balance_points - &wallet.frozen_points;

        Ok(WalletBalanceDto {
            user_id: wallet.user_id,
            balance_points: wallet.balance_points,
            frozen_points: wallet.frozen_points,
            available_balance,
            deposit_cny: wallet.deposit_cny,
            deposit_usd: wallet.deposit_usd,
            updated_at: wallet.updated_at,
        })
    }

    pub async fn get_transactions(
        &self,
        user_id: i32,
        req: crate::dto::common::PageRequest,
    ) -> Result<crate::dto::common::PageResponse<WalletTransactionDto>, ApiError> {
        let (transactions, total) = self
            .repo
            .get_transactions(user_id, req.page, req.page_size)
            .await
            .map_err(|_| ApiError::InternalServerError("Failed to get transactions".to_string()))?;

        let dtos = transactions
            .into_iter()
            .map(|t| self.transaction_to_dto(t, user_id))
            .collect();

        Ok(crate::dto::common::PageResponse::new(
            dtos,
            total,
            req.page,
            req.page_size,
        ))
    }

    fn transaction_to_dto(&self, t: WalletTransaction, user_id: i32) -> WalletTransactionDto {
        WalletTransactionDto {
            id: t.id,
            user_id,
            amount: t.amount,
            type_: t.type_,
            payment_method: t.payment_method,
            external_txn_id: t.external_txn_id,
            reference_id: t.reference_id.map(|id| id.to_string()),
            description: t.description,
            created_at: t.created_at,
        }
    }

    // ======================== XunhuPay Recharge ========================

    fn xunhupay(&self) -> Result<&XunhuPayClient, ApiError> {
        self.xunhupay
            .as_ref()
            .ok_or_else(|| ApiError::InternalServerError("Payment service not configured".into()))
    }

    fn notify_base_url(&self) -> Result<&str, ApiError> {
        self.notify_base_url
            .as_deref()
            .ok_or_else(|| ApiError::InternalServerError("API_BASE_URL is not configured".into()))
    }

    fn validate_amount_string(raw: &str) -> Result<BigDecimal, ApiError> {
        let amount = BigDecimal::from_str(raw)
            .map_err(|_| ApiError::BadRequest("Invalid amount format".into()))?;
        if amount <= BigDecimal::from(0) {
            return Err(ApiError::BadRequest("Amount must be positive".into()));
        }
        let trimmed = raw.trim();
        let mut parts = trimmed.split('.');
        let int_part = parts.next().unwrap_or_default();
        let frac_part = parts.next();
        if parts.next().is_some()
            || int_part.is_empty()
            || !int_part.chars().all(|c| c.is_ascii_digit())
            || frac_part
                .is_some_and(|frac| frac.len() > 2 || !frac.chars().all(|c| c.is_ascii_digit()))
        {
            return Err(ApiError::BadRequest(
                "Amount must be a positive decimal with at most 2 decimal places".into(),
            ));
        }
        if amount > BigDecimal::from(MAX_RECHARGE_CNY) {
            return Err(ApiError::BadRequest(format!(
                "Amount cannot exceed {} CNY",
                MAX_RECHARGE_CNY
            )));
        }
        Ok(amount)
    }

    fn amount_matches(expected: &BigDecimal, raw: &str) -> bool {
        BigDecimal::from_str(raw)
            .map(|actual| actual == *expected)
            .unwrap_or(false)
    }

    fn sanitize_return_url(&self, return_url: Option<String>) -> Result<Option<String>, ApiError> {
        let Some(return_url) = return_url else {
            return Ok(None);
        };
        let requested = reqwest::Url::parse(&return_url)
            .map_err(|_| ApiError::BadRequest("Invalid return_url".into()))?;
        if !matches!(requested.scheme(), "http" | "https") {
            return Err(ApiError::BadRequest(
                "return_url must use http or https".into(),
            ));
        }
        let base = reqwest::Url::parse(self.notify_base_url()?)
            .map_err(|_| ApiError::InternalServerError("API_BASE_URL is invalid".into()))?;
        if requested.host_str() != base.host_str() {
            return Err(ApiError::BadRequest(
                "return_url host is not allowed".into(),
            ));
        }
        Ok(Some(return_url))
    }

    fn payment_status_from_txn(txn: &WalletTransaction) -> PaymentStatus {
        PaymentStatus::from_db_value(txn.payment_status.as_deref())
            .unwrap_or(PaymentStatus::Pending)
    }

    fn recharge_channel_from_txn(txn: &WalletTransaction) -> Result<RechargeChannel, ApiError> {
        RechargeChannel::from_db_value(txn.payment_method.as_deref()).ok_or_else(|| {
            ApiError::InternalServerError(format!(
                "Invalid recharge channel stored for order {}",
                txn.external_txn_id.as_deref().unwrap_or("unknown")
            ))
        })
    }

    fn should_attempt_reconciliation(&self, txn: &WalletTransaction) -> bool {
        if Self::payment_status_from_txn(txn) != PaymentStatus::Pending {
            return false;
        }

        let age = Utc::now() - txn.created_at;
        age >= ChronoDuration::seconds(self.reconciliation_pending_age_secs)
    }

    fn payment_points(amount: &BigDecimal) -> BigDecimal {
        amount * BigDecimal::from(100)
    }

    fn exceeds_pending_order_limit(count: i64) -> bool {
        if MAX_PENDING_RECHARGE_ORDERS <= 0 {
            return false;
        }
        count >= MAX_PENDING_RECHARGE_ORDERS
    }

    fn validate_notification_against_order(
        &self,
        client: &XunhuPayClient,
        txn: &WalletTransaction,
        notification: &PayNotification,
    ) -> Result<(), ApiError> {
        if notification.appid != client.app_id() {
            return Err(ApiError::BadRequest("Payment appid mismatch".into()));
        }
        if !Self::amount_matches(&txn.amount, &notification.total_fee) {
            return Err(ApiError::BadRequest("Payment amount mismatch".into()));
        }
        if let Some(ref attach) = notification.attach {
            if attach != &txn.user_id.to_string() {
                return Err(ApiError::BadRequest("Payment attach mismatch".into()));
            }
        }
        if let Some(existing_open_order_id) = txn.open_order_id.as_deref() {
            if existing_open_order_id != notification.open_order_id {
                return Err(ApiError::BadRequest(
                    "Payment open_order_id mismatch".into(),
                ));
            }
        }
        Ok(())
    }

    fn validate_query_against_order(
        &self,
        txn: &WalletTransaction,
        query: &QueryResponse,
    ) -> Result<(), ApiError> {
        let Some(data) = query.data.as_ref() else {
            return Ok(());
        };
        if let Some(trade_order_id) = data.trade_order_id.as_deref() {
            if trade_order_id != txn.external_txn_id.as_deref().unwrap_or_default() {
                return Err(ApiError::BadRequest("Query trade_order_id mismatch".into()));
            }
        }
        if let Some(total_fee) = data.total_fee.as_deref() {
            if !Self::amount_matches(&txn.amount, total_fee) {
                return Err(ApiError::BadRequest("Query payment amount mismatch".into()));
            }
        }
        if let Some(existing_open_order_id) = txn.open_order_id.as_deref() {
            if let Some(query_open_order_id) = data.open_order_id.as_deref() {
                if existing_open_order_id != query_open_order_id {
                    return Err(ApiError::BadRequest("Query open_order_id mismatch".into()));
                }
            }
        }
        Ok(())
    }

    fn gateway_status_to_payment_status(
        current_status: PaymentStatus,
        gateway_status: XunhuPayOrderStatus,
    ) -> PaymentStatus {
        match gateway_status {
            XunhuPayOrderStatus::Pending => PaymentStatus::Pending,
            XunhuPayOrderStatus::Paid => PaymentStatus::Paid,
            XunhuPayOrderStatus::Refunding => PaymentStatus::Refunding,
            XunhuPayOrderStatus::Cancelled => PaymentStatus::Failed,
            XunhuPayOrderStatus::Refunded => PaymentStatus::Refunded,
            XunhuPayOrderStatus::RefundFailed => match current_status {
                PaymentStatus::Pending => PaymentStatus::Failed,
                PaymentStatus::Paid | PaymentStatus::Refunding => PaymentStatus::Paid,
                PaymentStatus::Failed => PaymentStatus::Failed,
                PaymentStatus::Refunded => PaymentStatus::Refunded,
            },
            XunhuPayOrderStatus::Unknown => PaymentStatus::Failed,
        }
    }

    fn notify_status_to_payment_status(
        current_status: PaymentStatus,
        raw_status: &str,
    ) -> PaymentStatus {
        Self::gateway_status_to_payment_status(
            current_status,
            XunhuPayOrderStatus::from_notify(raw_status),
        )
    }

    fn query_status_to_payment_status(
        current_status: PaymentStatus,
        raw_status: Option<&str>,
    ) -> PaymentStatus {
        Self::gateway_status_to_payment_status(
            current_status,
            XunhuPayOrderStatus::from_query(raw_status),
        )
    }

    fn should_refresh_status_from_gateway(&self, txn: &WalletTransaction) -> bool {
        match Self::payment_status_from_txn(txn) {
            PaymentStatus::Pending => self.should_attempt_reconciliation(txn),
            PaymentStatus::Paid | PaymentStatus::Refunding => true,
            PaymentStatus::Failed | PaymentStatus::Refunded => false,
        }
    }

    async fn fetch_transaction_by_order_no(
        &self,
        order_no: &str,
    ) -> Result<WalletTransaction, ApiError> {
        self.repo
            .find_transaction_by_external_id(order_no)
            .await
            .map_err(|_| ApiError::NotFound(format!("Order {order_no} not found")))
    }

    async fn finalize_paid_transaction(
        &self,
        txn: &WalletTransaction,
        platform_txn_id: &str,
        open_order_id: &str,
    ) -> Result<(), ApiError> {
        self.repo
            .atomic_confirm_payment(
                txn.id,
                txn.user_id,
                Self::payment_points(&txn.amount),
                platform_txn_id,
                open_order_id,
            )
            .await
            .map_err(|error| {
                ApiError::InternalServerError(format!("Atomic credit failed: {error}"))
            })?;
        Ok(())
    }

    async fn transition_transaction_status(
        &self,
        txn: &WalletTransaction,
        next_status: PaymentStatus,
        platform_txn_id: Option<&str>,
        open_order_id: Option<&str>,
    ) -> Result<(), ApiError> {
        let current_status = Self::payment_status_from_txn(txn);
        if current_status == next_status {
            return Ok(());
        }

        if !current_status.can_transition_to(next_status) {
            tracing::warn!(
                "Ignoring invalid recharge status transition for order {}: {} -> {}",
                txn.external_txn_id.as_deref().unwrap_or("unknown"),
                current_status.as_db_value(),
                next_status.as_db_value(),
            );
            return Ok(());
        }

        let platform_txn_id = platform_txn_id.or(txn.platform_txn_id.as_deref());
        let open_order_id = open_order_id.or(txn.open_order_id.as_deref());

        if next_status == PaymentStatus::Paid {
            if current_status == PaymentStatus::Pending {
                let platform_txn_id = platform_txn_id.ok_or_else(|| {
                    ApiError::InternalServerError(
                        "Missing platform transaction id for paid order".into(),
                    )
                })?;
                let open_order_id = open_order_id.ok_or_else(|| {
                    ApiError::InternalServerError("Missing open order id for paid order".into())
                })?;
                self.finalize_paid_transaction(txn, platform_txn_id, open_order_id)
                    .await?;
            } else {
                let _ = self
                    .repo
                    .update_payment_status_if_current(
                        txn.id,
                        current_status,
                        next_status,
                        platform_txn_id,
                        open_order_id,
                    )
                    .await
                    .map_err(|error| {
                        ApiError::InternalServerError(format!(
                            "Update payment status failed: {error}"
                        ))
                    })?;
            }
            return Ok(());
        }

        if next_status == PaymentStatus::Refunded
            && matches!(
                current_status,
                PaymentStatus::Paid | PaymentStatus::Refunding
            )
        {
            match self
                .repo
                .atomic_apply_refund(
                    txn.id,
                    txn.user_id,
                    current_status,
                    Self::payment_points(&txn.amount),
                    platform_txn_id,
                    open_order_id,
                )
                .await
                .map_err(|error| {
                    ApiError::InternalServerError(format!("Apply refund failed: {error}"))
                })? {
                RefundApplyOutcome::Applied | RefundApplyOutcome::Noop => {}
                RefundApplyOutcome::BlockedInsufficientBalance {
                    available_balance,
                    required_points,
                } => {
                    tracing::warn!(
                        "Blocking recharge refund for order {}: available_balance={} required_points={} current_status={}",
                        txn.external_txn_id.as_deref().unwrap_or("unknown"),
                        available_balance,
                        required_points,
                        current_status.as_db_value(),
                    );
                }
            }
            return Ok(());
        }

        let _ = self
            .repo
            .update_payment_status_if_current(
                txn.id,
                current_status,
                next_status,
                platform_txn_id,
                open_order_id,
            )
            .await
            .map_err(|error| {
                ApiError::InternalServerError(format!("Update payment status failed: {error}"))
            })?;
        Ok(())
    }

    async fn reconcile_transaction_with_query(
        &self,
        txn: WalletTransaction,
        query: QueryResponse,
    ) -> Result<WalletTransaction, ApiError> {
        let order_no = txn.external_txn_id.as_deref().unwrap_or("unknown");

        if query.errcode != 0 {
            tracing::warn!(
                "XunhuPay query errcode={} for order {}: {}",
                query.errcode,
                order_no,
                query.errmsg
            );
            return self.fetch_transaction_by_order_no(order_no).await;
        }

        let gateway_status = query.data.as_ref().and_then(|data| data.status.as_deref());

        tracing::info!(
            "XunhuPay query result for order {}: gateway_status={:?}, local_status={}",
            order_no,
            gateway_status,
            Self::payment_status_from_txn(&txn).as_db_value(),
        );

        if let Err(error) = self.validate_query_against_order(&txn, &query) {
            tracing::warn!(
                "Ignoring inconsistent query payload for order {}: {:?}",
                order_no,
                error
            );
            return self.fetch_transaction_by_order_no(order_no).await;
        }

        let current_status = Self::payment_status_from_txn(&txn);
        let next_status = Self::query_status_to_payment_status(current_status, gateway_status);

        if next_status == current_status {
            return Ok(txn);
        }

        tracing::info!(
            "Transitioning order {} from {} to {} based on query",
            order_no,
            current_status.as_db_value(),
            next_status.as_db_value(),
        );

        self.transition_transaction_status(
            &txn,
            next_status,
            query
                .data
                .as_ref()
                .and_then(|data| data.transaction_id.as_deref())
                .filter(|value| !value.is_empty()),
            query
                .data
                .as_ref()
                .and_then(|data| data.open_order_id.as_deref())
                .filter(|value| !value.is_empty()),
        )
        .await?;

        self.fetch_transaction_by_order_no(order_no).await
    }

    async fn reconcile_single_pending_transaction(
        &self,
        txn: WalletTransaction,
    ) -> Result<WalletTransaction, ApiError> {
        if self.xunhupay.is_none() {
            return Ok(txn);
        }

        if !self.should_attempt_reconciliation(&txn) {
            return Ok(txn);
        }

        let order_no = txn.external_txn_id.clone().ok_or_else(|| {
            ApiError::InternalServerError("Recharge order missing external_txn_id".into())
        })?;

        let query = match self
            .xunhupay()?
            .query_order(OrderId::TradeOrderId(order_no.clone()))
            .await
        {
            Ok(query) => query,
            Err(error) => {
                tracing::warn!(
                    "Recharge reconciliation query transport failed for order {order_no}: {error}"
                );
                return Ok(txn);
            }
        };

        self.reconcile_transaction_with_query(txn, query).await
    }

    async fn refresh_transaction_status_for_read(
        &self,
        txn: WalletTransaction,
    ) -> Result<WalletTransaction, ApiError> {
        if self.xunhupay.is_none() {
            return Ok(txn);
        }

        if !self.should_refresh_status_from_gateway(&txn) {
            return Ok(txn);
        }

        let order_no = txn.external_txn_id.clone().ok_or_else(|| {
            ApiError::InternalServerError("Recharge order missing external_txn_id".into())
        })?;

        let query = self
            .xunhupay()?
            .query_order(OrderId::TradeOrderId(order_no.clone()))
            .await
            .map_err(|error| {
                tracing::warn!("Recharge status query failed for order {order_no}: {error}");
                ApiError::PaymentServiceError("Recharge status query failed".into())
            });

        let Ok(query) = query else {
            return self.fetch_transaction_by_order_no(&order_no).await;
        };

        self.reconcile_transaction_with_query(txn, query).await
    }

    pub async fn reconcile_stale_pending_orders(&self) -> Result<usize, ApiError> {
        if self.xunhupay.is_none() {
            return Ok(0);
        }

        let cutoff = Utc::now() - ChronoDuration::seconds(self.reconciliation_pending_age_secs);
        let pending = self
            .repo
            .list_pending_recharge_transactions(cutoff, self.reconciliation_batch_size)
            .await
            .map_err(|error| {
                ApiError::InternalServerError(format!(
                    "List stale pending recharge orders failed: {error}"
                ))
            })?;

        let mut reconciled = 0usize;
        for txn in pending {
            let before = Self::payment_status_from_txn(&txn);
            let after = self.reconcile_single_pending_transaction(txn).await?;
            if before == PaymentStatus::Pending
                && Self::payment_status_from_txn(&after) != PaymentStatus::Pending
            {
                reconciled += 1;
            }
        }

        Ok(reconciled)
    }

    /// Create a recharge order via XunhuPay.
    ///
    /// Flow:
    /// 1. Validate channel (wechat / alipay)
    /// 2. Create a PENDING transaction in DB
    /// 3. Call XunhuPay pay API to get payment URL
    /// 4. Return payment URL to frontend
    pub async fn create_recharge(
        &self,
        user_id: i32,
        dto: RechargeRequestDto,
    ) -> Result<RechargeResponseDto, ApiError> {
        let client = self.xunhupay()?;

        if dto.channel == RechargeChannel::Wechat {
            return Err(ApiError::BadRequest(
                "WeChat Pay is temporarily unavailable".into(),
            ));
        }

        let amount_bd = Self::validate_amount_string(&dto.amount)?;
        let sanitized_return_url = self.sanitize_return_url(dto.return_url)?;
        if MAX_PENDING_RECHARGE_ORDERS > 0 {
            let pending_order_count = self
                .repo
                .count_pending_recharge_orders(user_id)
                .await
                .map_err(|error| {
                    ApiError::InternalServerError(format!(
                        "Count pending recharge orders failed: {error}"
                    ))
                })?;
            if Self::exceeds_pending_order_limit(pending_order_count) {
                return Err(ApiError::BadRequest(format!(
                    "Too many pending recharge orders (limit: {})",
                    MAX_PENDING_RECHARGE_ORDERS
                )));
            }
        }

        // trade_order_id max 32 chars per XunhuPay docs: 2 + 14 + 16 = 32
        let order_no = format!(
            "GM{}{}",
            chrono::Utc::now().format("%Y%m%d%H%M%S"),
            &uuid::Uuid::new_v4().simple().to_string()[..16]
        );

        let new_txn = NewWalletTransaction {
            user_id,
            amount: amount_bd,
            type_: "RECHARGE".to_string(),
            payment_method: Some(dto.channel.as_db_value().to_string()),
            external_txn_id: Some(order_no.clone()),
            reference_id: None,
            description: Some(format!(
                "Recharge ¥{} via {}",
                dto.amount,
                dto.channel.as_db_value()
            )),
            reference_type: None,
            payment_status: Some(PaymentStatus::Pending.as_db_value().to_string()),
        };
        let created_txn = self
            .repo
            .create_transaction(new_txn)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Create order failed: {e}")))?;

        // Call XunhuPay
        let notify_url = format!("{}/api/v1/public/payment/notify", self.notify_base_url()?);
        let pay_req = PayRequest {
            trade_order_id: order_no.clone(),
            total_fee: dto.amount,
            title: "GlanceMind Points Recharge".to_string(),
            notify_url,
            return_url: sanitized_return_url,
            callback_url: None,
            attach: Some(user_id.to_string()),
            wap_url: None,
            wap_name: Some("GlanceMind".to_string()),
            r#type: None,
        };

        let resp = client.pay(pay_req).await.map_err(|e| {
            tracing::error!("XunhuPay create order transport/parse failed: {}", e);
            ApiError::PaymentServiceError("Payment gateway request failed".into())
        });

        let resp = match resp {
            Ok(resp) => resp,
            Err(err) => {
                let _ = self
                    .repo
                    .update_payment_status_if_current(
                        created_txn.id,
                        PaymentStatus::Pending,
                        PaymentStatus::Failed,
                        None,
                        None,
                    )
                    .await;
                return Err(err);
            }
        };

        if resp.errcode != 0 {
            tracing::error!(
                "XunhuPay create order failed: errcode={}, errmsg={}",
                resp.errcode,
                resp.errmsg
            );
            let _ = self
                .repo
                .update_payment_status_if_current(
                    created_txn.id,
                    PaymentStatus::Pending,
                    PaymentStatus::Failed,
                    None,
                    None,
                )
                .await;
            return Err(ApiError::PaymentServiceError(
                "Payment gateway rejected the recharge order".into(),
            ));
        }

        Ok(RechargeResponseDto {
            order_no,
            payment_url: resp.url,
            qrcode_url: resp.url_qrcode,
        })
    }

    /// Handle XunhuPay async callback notification.
    ///
    /// Atomic: mark PAID + credit wallet in a single DB transaction.
    /// Idempotent: duplicate callbacks for an already-PAID order are no-ops.
    pub async fn handle_payment_notify(
        &self,
        notification: PayNotification,
    ) -> Result<(), ApiError> {
        let client = self.xunhupay()?;

        if !client.verify_notification(&notification) {
            tracing::warn!(
                "Payment notify signature verification failed for order {}",
                notification.trade_order_id
            );
            return Err(ApiError::BadRequest("Invalid signature".into()));
        }

        let txn = self
            .fetch_transaction_by_order_no(&notification.trade_order_id)
            .await?;

        self.validate_notification_against_order(client, &txn, &notification)?;

        let current_status = Self::payment_status_from_txn(&txn);
        if current_status.is_terminal() {
            tracing::warn!(
                "Ignoring notify for terminal recharge state order {}: {}",
                notification.trade_order_id,
                current_status.as_db_value(),
            );
            return Ok(());
        }

        let next_status =
            Self::notify_status_to_payment_status(current_status, &notification.status);
        self.transition_transaction_status(
            &txn,
            next_status,
            Some(&notification.transaction_id),
            Some(&notification.open_order_id),
        )
        .await?;

        tracing::info!(
            "Payment notify processed for order {}: user={}, next_status={}",
            notification.trade_order_id,
            txn.user_id,
            next_status.as_db_value(),
        );

        Ok(())
    }

    /// Query the status of a recharge order by order_no.
    pub async fn get_recharge_status(
        &self,
        order_no: &str,
        user_id: i32,
    ) -> Result<RechargeOrderStatusDto, ApiError> {
        let txn = self.fetch_transaction_by_order_no(order_no).await?;

        if txn.user_id != user_id {
            return Err(ApiError::NotFound(format!("Order {order_no} not found")));
        }

        let txn = self.refresh_transaction_status_for_read(txn).await?;

        Ok(RechargeOrderStatusDto {
            order_no: order_no.to_string(),
            status: Self::payment_status_from_txn(&txn),
            amount: txn.amount.to_string(),
            channel: Self::recharge_channel_from_txn(&txn)?,
            paid_at: txn.paid_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_status_transition_rules_are_strict() {
        assert!(PaymentStatus::Pending.can_transition_to(PaymentStatus::Paid));
        assert!(PaymentStatus::Pending.can_transition_to(PaymentStatus::Failed));
        assert!(PaymentStatus::Paid.can_transition_to(PaymentStatus::Refunding));
        assert!(PaymentStatus::Refunding.can_transition_to(PaymentStatus::Paid));
        assert!(!PaymentStatus::Failed.can_transition_to(PaymentStatus::Paid));
        assert!(!PaymentStatus::Refunded.can_transition_to(PaymentStatus::Paid));
    }

    #[test]
    fn notify_status_maps_refund_states_without_recrediting_pending_orders() {
        assert_eq!(
            WalletService::notify_status_to_payment_status(PaymentStatus::Pending, "OD"),
            PaymentStatus::Paid
        );
        assert_eq!(
            WalletService::notify_status_to_payment_status(PaymentStatus::Pending, "CD"),
            PaymentStatus::Refunded
        );
        assert_eq!(
            WalletService::notify_status_to_payment_status(PaymentStatus::Pending, "UD"),
            PaymentStatus::Failed
        );
        assert_eq!(
            WalletService::notify_status_to_payment_status(PaymentStatus::Refunding, "UD"),
            PaymentStatus::Paid
        );
        assert_eq!(
            WalletService::notify_status_to_payment_status(PaymentStatus::Pending, "UNKNOWN"),
            PaymentStatus::Failed
        );
    }

    #[test]
    fn query_status_maps_waiting_and_refund_states() {
        assert_eq!(
            WalletService::query_status_to_payment_status(PaymentStatus::Pending, Some("WP")),
            PaymentStatus::Pending
        );
        assert_eq!(
            WalletService::query_status_to_payment_status(PaymentStatus::Pending, Some("OD")),
            PaymentStatus::Paid
        );
        assert_eq!(
            WalletService::query_status_to_payment_status(PaymentStatus::Paid, Some("RD")),
            PaymentStatus::Refunding
        );
        assert_eq!(
            WalletService::query_status_to_payment_status(PaymentStatus::Pending, Some("CD")),
            PaymentStatus::Failed
        );
        assert_eq!(
            WalletService::query_status_to_payment_status(PaymentStatus::Paid, Some("CD")),
            PaymentStatus::Failed
        );
        assert_eq!(
            WalletService::notify_status_to_payment_status(PaymentStatus::Paid, "CD"),
            PaymentStatus::Refunded
        );
        assert_eq!(
            WalletService::query_status_to_payment_status(PaymentStatus::Refunding, Some("UD")),
            PaymentStatus::Paid
        );
    }

    #[test]
    fn validate_amount_string_rejects_too_large_amount() {
        let result = WalletService::validate_amount_string("10000.01");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Amount cannot exceed"));
    }

    #[test]
    fn validate_amount_string_accepts_max_amount() {
        let result = WalletService::validate_amount_string("10000.00");
        assert!(result.is_ok());
    }

    #[test]
    fn pending_order_limit_threshold_is_enforced() {
        assert!(!WalletService::exceeds_pending_order_limit(0));
        assert!(!WalletService::exceeds_pending_order_limit(4));
        assert!(!WalletService::exceeds_pending_order_limit(5));
        assert!(!WalletService::exceeds_pending_order_limit(6));
    }
}
