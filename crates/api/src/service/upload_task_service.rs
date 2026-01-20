use crate::config::database::Database;
use crate::dto::common::PageResponse;
use crate::dto::upload_task_dto::{
    CreateUploadTaskDto, DeviceTaskQueryDto, DeviceTaskResponseDto, UploadTaskResponseDto,
};
use crate::error::db_error::DbError;
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::repository::social_account_repository::SocialAccountRepository;
use crate::repository::upload_task_repository::UploadTaskRepository;
use chrono::Utc;
use diesel::result::Error as DieselError;
use glance_mind_db::entity::upload_task::{NewUploadTask, UpdateUploadTask};
use serde_json::json;
use std::sync::Arc;

#[derive(Clone)]
pub struct UploadTaskService {
    pub task_repo: UploadTaskRepository,
    pub account_repo: SocialAccountRepository,
}

impl UploadTaskService {
    pub fn new(db_conn: &Arc<Database>) -> Self {
        Self {
            task_repo: UploadTaskRepository::new(db_conn.pool.clone()),
            account_repo: SocialAccountRepository::new(db_conn.pool.clone()),
        }
    }

    pub async fn create_upload_task(
        &self,
        user_id: i32,
        payload: CreateUploadTaskDto,
    ) -> Result<UploadTaskResponseDto, ApiError> {
        // Verify that the social_account belongs to the user
        let social_account = self
            .account_repo
            .find_by_id(payload.social_account_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(
                    BusinessError::SocialAccountNotFound(payload.social_account_id),
                ),
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        if social_account.user_id != user_id {
            return Err(ApiError::BusinessError(BusinessError::TaskPermissionDenied));
        }

        // Enrich metadata with platform and profile_name from social_account
        let mut metadata = payload.metadata.clone();
        if let Some(obj) = metadata.as_object_mut() {
            // Get platform name - use user-selected platform_id if provided, otherwise use account's platform_id
            let platform_id_to_use = payload.platform_id.unwrap_or(social_account.platform_id);
            if let Ok(platform) = self
                .account_repo
                .get_platform_name(platform_id_to_use)
                .await
            {
                obj.insert("platform".to_string(), json!(platform));
            }

            // Add profile_name if available
            if let Some(ref profile_name) = social_account.profile_name {
                obj.insert("profile_name".to_string(), json!(profile_name));
            }
        }

        let new_task = NewUploadTask {
            user_id,
            social_account_id: payload.social_account_id,
            task_type: "upload".to_string(),
            metadata,
            status: "init".to_string(),
            platform_id: payload.platform_id,
        };

        let created_task = self
            .task_repo
            .create(new_task)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(UploadTaskResponseDto::from(created_task))
    }

    pub async fn get_tasks_by_device(
        &self,
        query: DeviceTaskQueryDto,
    ) -> Result<PageResponse<DeviceTaskResponseDto>, ApiError> {
        let (tasks, total) = self
            .task_repo
            .find_by_device_and_status(query.device_id, query.status, query.page, query.page_size)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let total_pages = ((total as f64) / (query.page_size as f64)).ceil() as i64;

        let list = tasks.into_iter().map(DeviceTaskResponseDto::from).collect();

        Ok(PageResponse {
            list,
            total,
            page: query.page,
            page_size: query.page_size,
            total_pages,
        })
    }

    pub async fn list_user_tasks(
        &self,
        user_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<PageResponse<UploadTaskResponseDto>, ApiError> {
        let (tasks, total) = self
            .task_repo
            .find_by_user_paginated(user_id, page, page_size)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let total_pages = ((total as f64) / (page_size as f64)).ceil() as i64;

        let list = tasks.into_iter().map(UploadTaskResponseDto::from).collect();

        Ok(PageResponse {
            list,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    pub async fn list_user_tasks_with_filter(
        &self,
        user_id: i32,
        page: i64,
        page_size: i64,
        status: Option<String>,
        platform_id: Option<i32>,
    ) -> Result<PageResponse<UploadTaskResponseDto>, ApiError> {
        let (tasks, total) = self
            .task_repo
            .find_by_user_with_filter(user_id, page, page_size, status, platform_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let total_pages = ((total as f64) / (page_size as f64)).ceil() as i64;

        let list = tasks.into_iter().map(UploadTaskResponseDto::from).collect();

        Ok(PageResponse {
            list,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    pub async fn update_task_status(
        &self,
        task_id: i32,
        user_id: i32,
        status: String,
    ) -> Result<UploadTaskResponseDto, ApiError> {
        // Verify task belongs to user
        let task = self
            .task_repo
            .find_by_id(task_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(BusinessError::TaskNotFound),
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        if task.user_id != user_id {
            return Err(ApiError::BusinessError(BusinessError::TaskPermissionDenied));
        }

        let update = UpdateUploadTask {
            status: Some(status),
            metadata: None,
            updated_at: Some(Utc::now()),
        };

        let updated_task = self
            .task_repo
            .update(task_id, update)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(UploadTaskResponseDto::from(updated_task))
    }

    /// Public method to update task status without user verification (for device/worker)
    pub async fn update_task_status_public(
        &self,
        task_id: i32,
        status: String,
    ) -> Result<crate::dto::upload_task_dto::UpdateTaskStatusResponse, ApiError> {
        // Find task first to verify it exists
        let _task = self
            .task_repo
            .find_by_id(task_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(BusinessError::TaskNotFound),
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        let update = UpdateUploadTask {
            status: Some(status),
            metadata: None,
            updated_at: Some(Utc::now()),
        };

        self.task_repo
            .update(task_id, update)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(crate::dto::upload_task_dto::UpdateTaskStatusResponse {
            success: true,
            message: "Task status updated successfully".to_string(),
        })
    }
}
