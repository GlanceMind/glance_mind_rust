use crate::config::database::Database;
use crate::dto::notification_dto::{NotificationDto, UnreadCountDto};
use crate::error::api_error::ApiError;
use crate::error::db_error::DbError;
use crate::repository::notification_repository::NotificationRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct NotificationService {
    notification_repo: NotificationRepository,
}

impl NotificationService {
    pub fn new(db_conn: &Arc<Database>) -> Self {
        Self {
            notification_repo: NotificationRepository::new(db_conn.pool.clone()),
        }
    }

    /// Get all notifications with read status for the user
    pub async fn get_notifications(&self, user_id: i32) -> Result<Vec<NotificationDto>, ApiError> {
        let notifications = self
            .notification_repo
            .find_all_active()
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let read_ids = self
            .notification_repo
            .find_read_ids_by_user(user_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let dtos = notifications
            .into_iter()
            .map(|n| {
                let is_read = read_ids.contains(&n.id);
                NotificationDto::from_entity(n, is_read)
            })
            .collect();

        Ok(dtos)
    }

    /// Get unread notification count
    pub async fn get_unread_count(&self, user_id: i32) -> Result<UnreadCountDto, ApiError> {
        let count = self
            .notification_repo
            .count_unread(user_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(UnreadCountDto { count })
    }

    /// Mark a single notification as read
    pub async fn mark_as_read(&self, user_id: i32, notification_id: i32) -> Result<(), ApiError> {
        self.notification_repo
            .mark_as_read(user_id, notification_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(())
    }

    /// Mark all notifications as read
    pub async fn mark_all_as_read(&self, user_id: i32) -> Result<(), ApiError> {
        self.notification_repo
            .mark_all_as_read(user_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(())
    }
}
