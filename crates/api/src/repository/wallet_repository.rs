use crate::config::database::DBPool;
use bigdecimal::BigDecimal;
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
            };

            let transaction = diesel::insert_into(wallet_transactions::table)
                .values(&new_transaction)
                .returning(WalletTransaction::as_returning())
                .get_result::<WalletTransaction>(conn)?;

            Ok((wallet, transaction))
        })
    }
}
