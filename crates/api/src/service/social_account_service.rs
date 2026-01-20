use crate::config::database::Database;
use crate::dto::social_account_dto::{
    CreateSocialAccountDto, SocialAccountDto, UpdateSocialAccountDto,
};
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::repository::social_account_repository::SocialAccountRepository;
use glance_mind_db::entity::social_account::{NewSocialAccount, SocialAccount};
use std::sync::Arc;

#[derive(Clone)]
pub struct SocialAccountService {
    repo: SocialAccountRepository,
}

impl SocialAccountService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            repo: SocialAccountRepository::new(db.pool.clone()),
        }
    }

    pub async fn list_accounts(
        &self,
        user_id: i32,
        req: crate::dto::common::PageRequest,
    ) -> Result<crate::dto::common::PageResponse<SocialAccountDto>, ApiError> {
        let (accounts, total) = self
            .repo
            .find_by_user(user_id, req.page, req.page_size)
            .await
            .map_err(|_| ApiError::InternalServerError("Failed to list accounts".to_string()))?;

        let dtos = accounts.into_iter().map(|a| self.to_dto(a)).collect();
        Ok(crate::dto::common::PageResponse::new(
            dtos,
            total,
            req.page,
            req.page_size,
        ))
    }

    pub async fn create_account(
        &self,
        user_id: i32,
        dto: CreateSocialAccountDto,
    ) -> Result<SocialAccountDto, ApiError> {
        // Get platform_id - for now use a mock ID
        // In real implementation, look up platform by name
        let platform_id = 1; // Mock ID

        let new_account = NewSocialAccount {
            user_id,
            platform_id,
            group_id: None,
            username: dto.username,
            cookie: dto.cookie.unwrap_or_default(), // Default empty string
            proxy_url: dto.proxy_url,
            status: "ACTIVE".to_string(),
            health_score: Some(100),
            daily_max_replies: dto.daily_max_replies.unwrap_or(50),
            device_id: dto.device_id,
            profile_name: dto.profile_name,
        };

        let account =
            self.repo.create(new_account).await.map_err(|_| {
                ApiError::InternalServerError("Failed to create account".to_string())
            })?;

        Ok(self.to_dto(account))
    }

    pub async fn update_account(
        &self,
        id: i32,
        user_id: i32,
        dto: UpdateSocialAccountDto,
    ) -> Result<SocialAccountDto, ApiError> {
        // Verify ownership
        let account = self
            .repo
            .find_by_id(id)
            .await
            .map_err(|_| ApiError::BusinessError(BusinessError::AccountNotFound))?;

        if account.user_id != user_id {
            return Err(ApiError::BusinessError(
                BusinessError::AccountPermissionDenied,
            ));
        }

        let mut updated = account.clone();
        if let Some(username) = dto.username {
            updated.username = username;
        }
        if let Some(cookie) = dto.cookie {
            updated.cookie = cookie;
        }
        if let Some(proxy) = dto.proxy_url {
            updated.proxy_url = Some(proxy);
        }
        if let Some(gid) = dto.group_id {
            if gid == 0 {
                updated.group_id = None;
            } else {
                updated.group_id = Some(gid);
            }
        }

        let changeset = glance_mind_db::entity::social_account::UpdateSocialAccount {
            group_id: Some(updated.group_id),
            username: Some(updated.username.clone()),
            cookie: Some(updated.cookie),
            proxy_url: Some(updated.proxy_url),
            status: dto.status,
            health_score: updated.health_score,
            updated_at: Some(chrono::Utc::now().naive_utc()),
            daily_max_replies: dto.daily_max_replies,
            device_id: dto.device_id.map(Some),
            profile_name: dto.profile_name.map(Some),
        };

        let result =
            self.repo.update(id, changeset).await.map_err(|_| {
                ApiError::InternalServerError("Failed to update account".to_string())
            })?;

        Ok(self.to_dto(result))
    }

    pub async fn verify_account(&self, id: i32, user_id: i32) -> Result<(), ApiError> {
        // Verify ownership
        let account = self
            .repo
            .find_by_id(id)
            .await
            .map_err(|_| ApiError::BusinessError(BusinessError::AccountNotFound))?;

        if account.user_id != user_id {
            return Err(ApiError::BusinessError(
                BusinessError::AccountPermissionDenied,
            ));
        }

        // Mock verification - in real system, this would test login
        Ok(())
    }

    pub async fn delete_account(&self, id: i32, user_id: i32) -> Result<(), ApiError> {
        // Verify ownership
        let account = self
            .repo
            .find_by_id(id)
            .await
            .map_err(|_| ApiError::BusinessError(BusinessError::AccountNotFound))?;

        if account.user_id != user_id {
            return Err(ApiError::BusinessError(
                BusinessError::AccountPermissionDenied,
            ));
        }

        self.repo
            .delete(id)
            .await
            .map_err(|_| ApiError::InternalServerError("Failed to delete account".to_string()))?;

        Ok(())
    }

    fn to_dto(&self, account: SocialAccount) -> SocialAccountDto {
        SocialAccountDto {
            id: account.id,
            group_id: account.group_id,
            username: account.username,
            cookie: Some(account.cookie),
            status: account.status,
            health_score: account.health_score,
            proxy_url: account.proxy_url,
            created_at: account.created_at,
            updated_at: account.updated_at,
            daily_max_replies: account.daily_max_replies,
            device_id: account.device_id,
            profile_name: account.profile_name,
        }
    }

    pub async fn get_statistics(
        &self,
        user_id: i32,
        group_id: Option<i32>,
    ) -> Result<crate::dto::social_account_dto::AccountStatisticsDto, ApiError> {
        self.repo
            .get_statistics(user_id, group_id)
            .await
            .map_err(|_| {
                ApiError::InternalServerError("Failed to get account statistics".to_string())
            })
    }
}
