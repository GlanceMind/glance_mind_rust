use crate::config::database::DBPool;
use crate::dto::wallet_dto::PaymentStatus;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::result::Error as DieselError;
use diesel::PgConnection as DieselPgConnection;
use diesel::SelectableHelper;
use glance_mind_db::entity::user_wallet::UserWallet;
use glance_mind_db::entity::wallet_transaction::{NewWalletTransaction, WalletTransaction};
use glance_mind_db::schema::{
    gm_user_wallets as user_wallets, gm_wallet_transactions as wallet_transactions,
};

pub type PgConnection = PooledConnection<ConnectionManager<DieselPgConnection>>;

#[derive(Clone)]
pub struct WalletRepository {
    pool: DBPool,
}

impl WalletRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    /// Static method to get wallet by user_id (for test usage)
    pub fn get_by_user_id(
        conn: &mut PgConnection,
        user_id: i32,
    ) -> Result<UserWallet, DieselError> {
        user_wallets::table
            .filter(user_wallets::user_id.eq(user_id))
            .first(conn)
    }

    pub async fn find_by_user(&self, user_id: i32) -> Result<UserWallet, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        user_wallets::table
            .filter(user_wallets::user_id.eq(user_id))
            .first(&mut conn)
    }

    pub async fn create(&self, user_id: i32) -> Result<UserWallet, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::insert_into(user_wallets::table)
            .values((
                user_wallets::user_id.eq(user_id),
                user_wallets::balance_points.eq(BigDecimal::from(0)),
                user_wallets::frozen_points.eq(BigDecimal::from(0)),
            ))
            .get_result(&mut conn)
    }

    pub async fn get_transactions(
        &self,
        user_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<WalletTransaction>, i64), DieselError> {
        let mut conn = self.pool.get().expect("Connection error");

        // Count total transactions
        let total = wallet_transactions::table
            .filter(wallet_transactions::user_id.eq(user_id))
            .count()
            .get_result(&mut conn)?;

        // Fetch paginated transactions
        let items = wallet_transactions::table
            .filter(wallet_transactions::user_id.eq(user_id))
            .select(WalletTransaction::as_select())
            .order(wallet_transactions::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .load(&mut conn)?;

        Ok((items, total))
    }

    /// Get a single transaction by ID (for authorization checks)
    pub async fn get_transaction_by_id(
        &self,
        transaction_id: i32,
    ) -> Result<WalletTransaction, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        wallet_transactions::table
            .find(transaction_id)
            .select(WalletTransaction::as_select())
            .first(&mut conn)
    }

    pub async fn create_transaction(
        &self,
        transaction: NewWalletTransaction,
    ) -> Result<WalletTransaction, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::insert_into(wallet_transactions::table)
            .values(&transaction)
            .returning(WalletTransaction::as_returning())
            .get_result(&mut conn)
    }

    #[allow(dead_code)]
    pub async fn update_balance(
        &self,
        user_id: i32,
        balance_delta: BigDecimal,
    ) -> Result<UserWallet, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::update(user_wallets::table)
            .filter(user_wallets::user_id.eq(user_id))
            .set(user_wallets::balance_points.eq(user_wallets::balance_points + balance_delta))
            .returning(UserWallet::as_returning())
            .get_result(&mut conn)
    }

    /// Freeze campaign budget in wallet (increase frozen_points)
    /// This should be called in a transaction with campaign status update
    pub async fn freeze_campaign_budget(
        &self,
        user_id: i32,
        amount: &BigDecimal,
    ) -> Result<UserWallet, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::update(user_wallets::table)
            .filter(user_wallets::user_id.eq(user_id))
            .set(user_wallets::frozen_points.eq(user_wallets::frozen_points + amount))
            .returning(UserWallet::as_returning())
            .get_result(&mut conn)
    }

    /// Unfreeze campaign budget in wallet (decrease frozen_points)
    /// This should be called when campaign ends
    pub async fn unfreeze_campaign_budget(
        &self,
        user_id: i32,
        amount: &BigDecimal,
    ) -> Result<UserWallet, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::update(user_wallets::table)
            .filter(user_wallets::user_id.eq(user_id))
            .set(user_wallets::frozen_points.eq(user_wallets::frozen_points - amount))
            .returning(UserWallet::as_returning())
            .get_result(&mut conn)
    }

    /// Check if user has enough available balance for campaign
    /// Available balance = balance_points - frozen_points
    pub async fn validate_available_balance(
        &self,
        user_id: i32,
        required_amount: &BigDecimal,
    ) -> Result<bool, DieselError> {
        let wallet = self.find_by_user(user_id).await?;
        let available = &wallet.balance_points - &wallet.frozen_points;
        Ok(available >= *required_amount)
    }

    /// Find a transaction by its external_txn_id (merchant order number)
    pub async fn find_transaction_by_external_id(
        &self,
        external_id: &str,
    ) -> Result<WalletTransaction, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        wallet_transactions::table
            .filter(wallet_transactions::external_txn_id.eq(external_id))
            .select(WalletTransaction::as_select())
            .first(&mut conn)
    }

    /// List stale pending recharge orders for reconciliation.
    pub async fn list_pending_recharge_transactions(
        &self,
        older_than: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<WalletTransaction>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        wallet_transactions::table
            .filter(wallet_transactions::type_.eq("RECHARGE"))
            .filter(
                wallet_transactions::payment_status.eq(Some(PaymentStatus::Pending.as_db_value())),
            )
            .filter(wallet_transactions::created_at.le(older_than))
            .select(WalletTransaction::as_select())
            .order(wallet_transactions::created_at.asc())
            .limit(limit)
            .load(&mut conn)
    }

    /// Transition a recharge order from one explicit payment state to another.
    ///
    /// `PENDING -> PAID` must still go through `atomic_confirm_payment()` so the
    /// wallet credit and order transition stay in a single database transaction.
    pub async fn update_payment_status_if_current(
        &self,
        txn_id: i32,
        current_status: PaymentStatus,
        next_status: PaymentStatus,
        platform_txn_id: Option<&str>,
        open_order_id: Option<&str>,
    ) -> Result<Option<WalletTransaction>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::update(
            wallet_transactions::table
                .filter(wallet_transactions::id.eq(txn_id))
                .filter(wallet_transactions::payment_status.eq(Some(current_status.as_db_value()))),
        )
        .set((
            wallet_transactions::payment_status.eq(Some(next_status.as_db_value())),
            wallet_transactions::platform_txn_id.eq(platform_txn_id),
            wallet_transactions::open_order_id.eq(open_order_id),
        ))
        .returning(WalletTransaction::as_returning())
        .get_result(&mut conn)
        .optional()
    }

    /// Atomically: mark order PAID + credit wallet + write deposit record.
    ///
    /// Uses a single DB transaction to prevent "paid but not credited" states.
    /// The WHERE clause on payment_status='PENDING' acts as a row-level lock
    /// that makes concurrent duplicate callbacks safe.
    pub async fn atomic_confirm_payment(
        &self,
        txn_id: i32,
        user_id: i32,
        points: BigDecimal,
        platform_txn_id: &str,
        open_order_id: &str,
    ) -> Result<(), DieselError> {
        let mut conn = self.pool.get().expect("Connection error");

        conn.transaction(|conn| {
            let recharge_order = wallet_transactions::table
                .find(txn_id)
                .select(WalletTransaction::as_select())
                .first::<WalletTransaction>(conn)?;

            // CAS: only transition PENDING -> PAID; concurrent calls will match 0 rows
            let updated = diesel::update(
                wallet_transactions::table
                    .filter(wallet_transactions::id.eq(txn_id))
                    .filter(
                        wallet_transactions::payment_status
                            .eq(Some(PaymentStatus::Pending.as_db_value())),
                    ),
            )
            .set((
                wallet_transactions::payment_status.eq(Some(PaymentStatus::Paid.as_db_value())),
                wallet_transactions::platform_txn_id.eq(Some(platform_txn_id)),
                wallet_transactions::open_order_id.eq(Some(open_order_id)),
                wallet_transactions::paid_at.eq(diesel::dsl::now),
            ))
            .execute(conn)?;

            if updated == 0 {
                // Already processed or not in PENDING state — idempotent no-op
                return Ok(());
            }

            // Ensure wallet exists
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

            // Credit balance
            diesel::update(user_wallets::table)
                .filter(user_wallets::user_id.eq(user_id))
                .set(user_wallets::balance_points.eq(user_wallets::balance_points + &points))
                .execute(conn)?;

            // Write DEPOSIT transaction record for audit trail
            let deposit_txn = NewWalletTransaction {
                user_id,
                amount: points,
                type_: "DEPOSIT".to_string(),
                payment_method: recharge_order.payment_method.clone(),
                external_txn_id: None,
                reference_id: Some(txn_id),
                description: Some(format!(
                    "Wallet credit for recharge order {}",
                    recharge_order
                        .external_txn_id
                        .as_deref()
                        .unwrap_or("unknown_order")
                )),
                reference_type: Some("recharge".to_string()),
                payment_status: None,
            };
            diesel::insert_into(wallet_transactions::table)
                .values(&deposit_txn)
                .execute(conn)?;

            Ok(())
        })
    }

    /// Add balance and create transaction (for welcome bonus, deposits, etc.)
    pub async fn add_balance_with_transaction(
        &self,
        user_id: i32,
        amount: BigDecimal,
        transaction_type: String,
        description: String,
    ) -> Result<(UserWallet, WalletTransaction), DieselError> {
        let mut conn = self.pool.get().expect("Connection error");

        // Start transaction
        conn.transaction(|conn| {
            // Ensure wallet exists, create if not
            let wallet_exists = user_wallets::table
                .filter(user_wallets::user_id.eq(user_id))
                .select(user_wallets::user_id)
                .first::<i32>(conn)
                .optional()?;

            if wallet_exists.is_none() {
                // Create wallet with zero balance (will be updated below)
                diesel::insert_into(user_wallets::table)
                    .values((
                        user_wallets::user_id.eq(user_id),
                        user_wallets::balance_points.eq(BigDecimal::from(0)),
                        user_wallets::frozen_points.eq(BigDecimal::from(0)),
                    ))
                    .execute(conn)?;
            }

            // Update balance (works whether wallet was just created or already existed)
            let wallet = diesel::update(user_wallets::table)
                .filter(user_wallets::user_id.eq(user_id))
                .set(user_wallets::balance_points.eq(user_wallets::balance_points + &amount))
                .returning(UserWallet::as_returning())
                .get_result::<UserWallet>(conn)?;

            // Create transaction record
            let new_transaction = NewWalletTransaction {
                user_id,
                amount: amount.clone(),
                type_: transaction_type,
                payment_method: None,
                external_txn_id: None,
                reference_id: None,
                description: Some(description),
                reference_type: None,
                payment_status: None,
            };

            let transaction = diesel::insert_into(wallet_transactions::table)
                .values(&new_transaction)
                .returning(WalletTransaction::as_returning())
                .get_result::<WalletTransaction>(conn)?;

            Ok((wallet, transaction))
        })
    }
}
