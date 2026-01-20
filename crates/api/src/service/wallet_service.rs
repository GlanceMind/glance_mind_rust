use crate::config::database::Database;
use crate::dto::wallet_dto::{
    TopUpRequestDto, TopUpResponseDto, WalletBalanceDto, WalletTransactionDto,
};
use glance_mind_db::entity::wallet_transaction::{NewWalletTransaction, WalletTransaction};
use crate::error::api_error::ApiError;
use crate::repository::wallet_repository::WalletRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct WalletService {
    repo: WalletRepository,
}

impl WalletService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            repo: WalletRepository::new(db.pool.clone()),
        }
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

    pub async fn create_recharge_order(
        &self,
        user_id: i32,
        dto: TopUpRequestDto,
    ) -> Result<TopUpResponseDto, ApiError> {
        let _wallet = self
            .repo
            .find_by_user(user_id)
            .await
            .map_err(|_| ApiError::InternalServerError("Failed to find wallet".to_string()))?;

        let new_transaction = NewWalletTransaction {
            user_id,
            amount: dto.amount.clone(),
            type_: "RECHARGE".to_string(),
            payment_method: Some(dto.payment_method.clone()),
            external_txn_id: None,
            reference_id: None,
            description: Some(format!("Top up via {}", dto.payment_method)),
        };

        let transaction = self
            .repo
            .create_transaction(new_transaction)
            .await
            .map_err(|_| ApiError::InternalServerError("Failed to create order".to_string()))?;

        let payment_url = format!("https://payment.example.com/pay?txn_id={}", transaction.id);

        Ok(TopUpResponseDto {
            transaction_id: transaction.id,
            payment_url: Some(payment_url),
            message: "Order created successfully".to_string(),
        })
    }

    pub async fn get_order_status(
        &self,
        order_id: i32,
        _user_id: i32,
    ) -> Result<TopUpResponseDto, ApiError> {
        // Mock implementation - in real system, query payment gateway
        Ok(TopUpResponseDto {
            transaction_id: order_id,
            payment_url: None,
            message: "Order status is PENDING".to_string(),
        })
    }

    fn transaction_to_dto(&self, t: WalletTransaction, user_id: i32) -> WalletTransactionDto {
        WalletTransactionDto {
            id: t.id,
            user_id, // Use passed user_id
            amount: t.amount,
            type_: t.type_,
            payment_method: t.payment_method,
            external_txn_id: t.external_txn_id,
            reference_id: t.reference_id.map(|id| id.to_string()),
            description: t.description,
            created_at: t.created_at,
        }
    }
}
