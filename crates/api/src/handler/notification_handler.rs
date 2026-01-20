use axum::{
    extract::{Extension, Path, State},
    Json,
};

use crate::{
    config::database::Database,
    dto::notification_dto::{NotificationDto, UnreadCountDto},
    error::api_error::ApiError,
    service::notification_service::NotificationService,
};
use glance_mind_db::entity::user::User;
use std::sync::Arc;

/// Get all notifications with read status
pub async fn get_notifications(
    Extension(user): Extension<User>,
    State(db): State<Arc<Database>>,
) -> Result<Json<Vec<NotificationDto>>, ApiError> {
    let service = NotificationService::new(&db);
    let notifications = service.get_notifications(user.id).await?;
    Ok(Json(notifications))
}

/// Get unread notification count
pub async fn get_unread_count(
    Extension(user): Extension<User>,
    State(db): State<Arc<Database>>,
) -> Result<Json<UnreadCountDto>, ApiError> {
    let service = NotificationService::new(&db);
    let count = service.get_unread_count(user.id).await?;
    Ok(Json(count))
}

/// Mark a single notification as read
pub async fn mark_as_read(
    Extension(user): Extension<User>,
    State(db): State<Arc<Database>>,
    Path(notification_id): Path<i32>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = NotificationService::new(&db);
    service.mark_as_read(user.id, notification_id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// Mark all notifications as read
pub async fn mark_all_as_read(
    Extension(user): Extension<User>,
    State(db): State<Arc<Database>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = NotificationService::new(&db);
    service.mark_all_as_read(user.id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}
