use crate::config::database::Database;
use crate::dto::social_account_dto::{CreateSocialGroupDto, SocialGroupDto, UpdateSocialGroupDto};
use glance_mind_db::entity::social_group::{NewSocialGroup, SocialGroup};
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::repository::social_group_repository::SocialGroupRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct SocialGroupService {
    repo: SocialGroupRepository,
}

impl SocialGroupService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            repo: SocialGroupRepository::new(db.pool.clone()),
        }
    }

    pub async fn list_groups(
        &self,
        user_id: i32,
        req: crate::dto::common::PageRequest,
    ) -> Result<crate::dto::common::PageResponse<SocialGroupDto>, ApiError> {
        let (groups, total) = self
            .repo
            .find_by_user(user_id, req.page, req.page_size)
            .await
            .map_err(|_| ApiError::InternalServerError("Failed to list groups".to_string()))?;

        let dtos = groups.into_iter().map(Self::to_dto).collect();
        Ok(crate::dto::common::PageResponse::new(
            dtos,
            total,
            req.page,
            req.page_size,
        ))
    }

    pub async fn create_group(
        &self,
        user_id: i32,
        dto: CreateSocialGroupDto,
    ) -> Result<SocialGroupDto, ApiError> {
        let new_group = NewSocialGroup {
            user_id,
            platform_id: dto.platform_id,
            group_name: dto.group_name,
        };

        let group = self
            .repo
            .create(new_group)
            .await
            .map_err(|_| ApiError::InternalServerError("Failed to create group".to_string()))?;

        Ok(Self::to_dto(group))
    }

    pub async fn update_group(
        &self,
        id: i32,
        user_id: i32,
        dto: UpdateSocialGroupDto,
    ) -> Result<SocialGroupDto, ApiError> {
        // Verify ownership
        let group = self
            .repo
            .find_by_id(id, user_id)
            .await
            .map_err(|_| ApiError::BusinessError(BusinessError::GroupNotFound))?;

        if group.user_id != user_id {
            return Err(ApiError::BusinessError(BusinessError::GroupNotFound));
        }

        let updated = self
            .repo
            .update(id, user_id, &dto.group_name)
            .await
            .map_err(|_| ApiError::InternalServerError("Failed to update group".to_string()))?;

        Ok(Self::to_dto(updated))
    }

    pub async fn delete_group(&self, id: i32, user_id: i32) -> Result<(), ApiError> {
        // Verify ownership
        let group = self
            .repo
            .find_by_id(id, user_id)
            .await
            .map_err(|_| ApiError::BusinessError(BusinessError::GroupNotFound))?;

        if group.user_id != user_id {
            return Err(ApiError::BusinessError(BusinessError::GroupNotFound));
        }

        self.repo
            .delete(id, user_id)
            .await
            .map_err(|_| ApiError::InternalServerError("Failed to delete group".to_string()))?;

        Ok(())
    }

    fn to_dto(group: SocialGroup) -> SocialGroupDto {
        SocialGroupDto {
            id: group.id,
            user_id: group.user_id,
            platform_id: group.platform_id,
            group_name: group.group_name,
            accounts: None, // Accounts can be populated separately if needed
            created_at: group.created_at,
            updated_at: group.updated_at,
        }
    }
}
