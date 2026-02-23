use crate::api_ok;
use crate::dto::dm_dto::*;
use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use axum::{
    extract::{Extension, Path, Query},
    response::IntoResponse,
    Json,
};
use glance_mind_db::entity::user::User;

/// GET /dm/conversations
/// Returns conversation list + device online status.
pub async fn list_conversations(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Query(query): Query<DmConversationsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = state
        .nats_dm_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalServerError("NATS DM service not available".into()))?;
    let response = nats_dm.list_conversations(user.id, query).await?;
    Ok(api_ok!(response))
}

/// GET /dm/conversations/:conv_id/messages
/// Returns message history for a conversation.
pub async fn get_messages(
    Extension(_user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(conv_id): Path<String>,
    Query(query): Query<DmMessagesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = state
        .nats_dm_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalServerError("NATS DM service not available".into()))?;

    // Verify the conv_id belongs to this user by checking that the social_account_id
    // in the conv_id belongs to a social account owned by this user.
    // conv_id format: {social_account_id}_{remote_user_id}
    // For now we trust the conv_id and let NATS return empty if not found.

    let response = nats_dm.get_messages(&conv_id, query).await?;
    Ok(api_ok!(response))
}

/// POST /dm/conversations/:conv_id/reply
/// Sends a reply command to the executor via NATS.
pub async fn send_reply(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(conv_id): Path<String>,
    Json(req): Json<DmReplyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = state
        .nats_dm_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalServerError("NATS DM service not available".into()))?;

    // Parse conv_id to get social_account_id
    let parts: Vec<&str> = conv_id.splitn(2, '_').collect();
    if parts.len() != 2 {
        return Err(ApiError::BadRequest("Invalid conv_id format".into()));
    }
    let social_account_id: i32 = parts[0]
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid social_account_id in conv_id".into()))?;
    let remote_username = parts[1];

    // Look up the social account from PG to get device_id and profile_name
    let account = state
        .social_account_service
        .get_account_by_id(social_account_id, user.id)
        .await?;

    let device_id = account
        .device_id
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Account has no device_id assigned".into()))?;
    let profile_name = account
        .profile_name
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Account has no profile_name assigned".into()))?;

    let response = nats_dm
        .send_reply(
            &conv_id,
            device_id,
            social_account_id,
            account.platform_id,
            profile_name,
            remote_username,
            &req.content,
            &req.content_type,
        )
        .await?;
    Ok(api_ok!(response))
}

/// POST /dm/conversations/:conv_id/read
/// Marks a conversation as read (unread_count = 0).
pub async fn mark_read(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(conv_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = state
        .nats_dm_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalServerError("NATS DM service not available".into()))?;
    nats_dm.mark_read(user.id, &conv_id).await?;
    Ok(api_ok!(msg: "Marked as read", "已标记为已读"))
}

/// PUT /dm/conversations/:conv_id/settings
/// Updates conversation settings (reply_mode, status).
pub async fn update_settings(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(conv_id): Path<String>,
    Json(req): Json<DmSettingsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = state
        .nats_dm_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalServerError("NATS DM service not available".into()))?;
    nats_dm.update_settings(user.id, &conv_id, req).await?;
    Ok(api_ok!(msg: "Settings updated", "设置已更新"))
}

/// GET /dm/stats
/// Returns DM statistics (total conversations, unread counts per platform).
pub async fn get_stats(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = state
        .nats_dm_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalServerError("NATS DM service not available".into()))?;
    let stats = nats_dm.get_stats(user.id).await?;
    Ok(api_ok!(stats))
}

/// GET /dm/monitor-config/:device_id
/// Returns the DM monitor configuration for a device.
pub async fn get_monitor_config(
    Extension(_user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(device_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = state
        .nats_dm_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalServerError("NATS DM service not available".into()))?;
    let config = nats_dm.get_monitor_config(&device_id).await?;
    Ok(api_ok!(config))
}

/// PUT /dm/monitor-config/:device_id
/// Updates the DM monitor configuration for a device.
pub async fn update_monitor_config(
    Extension(_user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(device_id): Path<String>,
    Json(req): Json<DmMonitorConfigUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = state
        .nats_dm_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalServerError("NATS DM service not available".into()))?;
    let config = nats_dm.update_monitor_config(&device_id, req).await?;
    Ok(api_ok!(config))
}

/// GET /dm/nats-token
/// Generates a restricted NATS JWT for frontend WebSocket connection.
pub async fn get_nats_token(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = state
        .nats_dm_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalServerError("NATS DM service not available".into()))?;
    let response = nats_dm.generate_nats_token(user.id).await?;
    Ok(api_ok!(response))
}
