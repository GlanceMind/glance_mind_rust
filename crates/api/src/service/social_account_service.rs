use crate::config::database::Database;
use crate::dto::social_account_dto::{
    BatchCreateAccountsDto, BatchCreateResultDto, CreateSocialAccountDto, SocialAccountDto,
    UpdateSocialAccountDto,
};
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::repository::social_account_repository::SocialAccountRepository;
use crate::repository::social_group_repository::SocialGroupRepository;
use crate::service::validation::group_platform::load_and_check_group;
use glance_mind_db::entity::social_account::{NewSocialAccount, SocialAccount};
use std::sync::Arc;
use tracing::{error, info};

#[derive(Clone)]
pub struct SocialAccountService {
    repo: SocialAccountRepository,
    group_repo: SocialGroupRepository,
}

impl SocialAccountService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            repo: SocialAccountRepository::new(db.pool.clone()),
            group_repo: SocialGroupRepository::new(db.pool.clone()),
        }
    }

    pub async fn list_accounts(
        &self,
        user_id: i32,
        req: crate::dto::social_account_dto::AccountListRequest,
    ) -> Result<crate::dto::common::PageResponse<SocialAccountDto>, ApiError> {
        let (accounts, total) = self
            .repo
            .find_by_user(
                user_id,
                req.page,
                req.page_size,
                req.group_id,
                req.username,
                req.platform_id,
                req.status,
                req.device_id,
            )
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

    /// Get a single social account by ID, verifying user ownership.
    pub async fn get_account_by_id(
        &self,
        account_id: i32,
        user_id: i32,
    ) -> Result<SocialAccount, ApiError> {
        let account =
            self.repo.find_by_id(account_id).await.map_err(|_| {
                ApiError::NotFound(format!("Social account {} not found", account_id))
            })?;
        if account.user_id != user_id {
            return Err(ApiError::Forbidden("Not your account".into()));
        }
        Ok(account)
    }

    pub async fn create_account(
        &self,
        user_id: i32,
        dto: CreateSocialAccountDto,
    ) -> Result<SocialAccountDto, ApiError> {
        let platform_id = dto.platform_id;

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
        // Apply the platform change first so any group provided in the same
        // request is validated against the NEW platform, not the old one.
        if let Some(pid) = dto.platform_id {
            updated.platform_id = pid;
        }
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
                load_and_check_group(&self.group_repo, gid, user_id, updated.platform_id).await?;
                updated.group_id = Some(gid);
            }
        }

        let changeset = glance_mind_db::entity::social_account::UpdateSocialAccount {
            platform_id: Some(updated.platform_id),
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

    /// Remove account from group (set group_id to NULL)
    pub async fn remove_from_group(&self, id: i32, user_id: i32) -> Result<(), ApiError> {
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

        self.repo.clear_group(id).await.map_err(|_| {
            ApiError::InternalServerError("Failed to remove from group".to_string())
        })?;

        Ok(())
    }

    fn to_dto(&self, account: SocialAccount) -> SocialAccountDto {
        SocialAccountDto {
            id: account.id,
            platform_id: account.platform_id,
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

    /// Batch create multiple social accounts
    /// Maximum 100 accounts per request
    pub async fn batch_create_accounts(
        &self,
        user_id: i32,
        dto: BatchCreateAccountsDto,
    ) -> Result<BatchCreateResultDto, ApiError> {
        // Parse profile range
        let (prefix, start_num, end_num) =
            self.parse_profile_range(&dto.profile_start, &dto.profile_end)?;

        let total_count = end_num - start_num + 1;

        // Validate: max 100 accounts per batch
        if total_count > 100 {
            return Err(ApiError::BadRequest(
                "Maximum 100 accounts can be created in a single batch".to_string(),
            ));
        }

        if total_count <= 0 {
            return Err(ApiError::BadRequest(
                "Invalid profile range: start must be less than or equal to end".to_string(),
            ));
        }

        // Normalize the 0-sentinel: treat group_id=0 as "no group" (NULL in DB).
        // Validate via load_and_check_group only when a real group id is provided.
        let group_id = dto.group_id.filter(|&g| g != 0);
        if let Some(gid) = group_id {
            load_and_check_group(&self.group_repo, gid, user_id, dto.platform_id).await?;
        }

        info!(
            "Batch creating {} accounts for user {} with prefix '{}' from {} to {}",
            total_count, user_id, prefix, start_num, end_num
        );

        // Generate candidate accounts first (preserve request order)
        let mut candidates: Vec<NewSocialAccount> = Vec::with_capacity(total_count as usize);
        let mut candidate_profiles: Vec<String> = Vec::with_capacity(total_count as usize);
        for i in start_num..=end_num {
            let profile_name = format!("{}{}", prefix, i);
            let username = format!("{}_{}", dto.username, profile_name);

            candidates.push(NewSocialAccount {
                user_id,
                platform_id: dto.platform_id,
                group_id,
                username,
                cookie: String::new(),
                proxy_url: None,
                status: "ACTIVE".to_string(),
                health_score: Some(100),
                daily_max_replies: dto.daily_max_replies,
                device_id: dto.device_id.clone(),
                profile_name: Some(profile_name.clone()),
            });
            candidate_profiles.push(profile_name);
        }

        // Pre-query existing profile_names for this (user, platform) so we can
        // skip duplicates. There is no UNIQUE constraint on the table, so this
        // is best-effort dedupe (not race-proof across concurrent callers, but
        // good enough for a user-facing "retry batch" flow).
        let existing: std::collections::HashSet<String> = self
            .repo
            .find_existing_profile_names(user_id, dto.platform_id, &candidate_profiles)
            .await
            .map_err(|e| {
                error!("Failed to query existing profile_names: {:?}", e);
                ApiError::InternalServerError("Failed to check existing accounts".to_string())
            })?
            .into_iter()
            .collect();

        let mut new_accounts: Vec<NewSocialAccount> = Vec::with_capacity(candidates.len());
        let mut skipped_profiles: Vec<String> = Vec::new();
        for acc in candidates {
            match &acc.profile_name {
                Some(name) if existing.contains(name) => {
                    skipped_profiles.push(name.clone());
                }
                _ => new_accounts.push(acc),
            }
        }

        if new_accounts.is_empty() {
            info!(
                "All {} profiles already exist for user {} on platform {}; nothing to insert",
                skipped_profiles.len(),
                user_id,
                dto.platform_id
            );
            return Ok(BatchCreateResultDto {
                created_count: 0,
                total_attempted: total_count,
                skipped_count: skipped_profiles.len() as i32,
                skipped_profiles,
                created_ids: vec![],
                errors: vec![],
            });
        }

        // Batch insert the survivors
        match self.repo.batch_create(new_accounts).await {
            Ok(accounts) => {
                let created_ids: Vec<i32> = accounts.iter().map(|a| a.id).collect();
                info!(
                    "Batch create: inserted {}, skipped {} duplicates",
                    accounts.len(),
                    skipped_profiles.len()
                );

                Ok(BatchCreateResultDto {
                    created_count: accounts.len() as i32,
                    total_attempted: total_count,
                    skipped_count: skipped_profiles.len() as i32,
                    skipped_profiles,
                    created_ids,
                    errors: vec![],
                })
            }
            Err(e) => {
                error!("Failed to batch create accounts: {:?}", e);
                Err(ApiError::InternalServerError(
                    "Failed to batch create accounts".to_string(),
                ))
            }
        }
    }

    /// Parse profile range like "account_1" to "account_100"
    /// Returns (prefix, start_number, end_number)
    fn parse_profile_range(&self, start: &str, end: &str) -> Result<(String, i32, i32), ApiError> {
        // Extract prefix and number from start
        let start_match = start.rfind(|c: char| !c.is_ascii_digit());
        let end_match = end.rfind(|c: char| !c.is_ascii_digit());

        let (start_prefix, start_num_str) = if let Some(idx) = start_match {
            (&start[..=idx], &start[idx + 1..])
        } else {
            return Err(ApiError::BadRequest(
                "Invalid profile_start format. Expected format like 'account_1'".to_string(),
            ));
        };

        let (end_prefix, end_num_str) = if let Some(idx) = end_match {
            (&end[..=idx], &end[idx + 1..])
        } else {
            return Err(ApiError::BadRequest(
                "Invalid profile_end format. Expected format like 'account_100'".to_string(),
            ));
        };

        // Verify prefixes match
        if start_prefix != end_prefix {
            return Err(ApiError::BadRequest(
                "Profile start and end must have the same prefix".to_string(),
            ));
        }

        // Parse numbers
        let start_num: i32 = start_num_str
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid number in profile_start".to_string()))?;

        let end_num: i32 = end_num_str
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid number in profile_end".to_string()))?;

        Ok((start_prefix.to_string(), start_num, end_num))
    }
}
