use crate::api_ok;
use crate::dto::dm_dto::*;
use crate::error::api_error::ApiError;
use crate::service::nats_dm_service::NatsDmService;
use crate::state::user_state::UserState;
use axum::{
    extract::{Extension, Path, Query},
    response::IntoResponse,
    Json,
};
use glance_mind_db::entity::user::User;

fn require_nats_dm(state: &UserState) -> Result<&NatsDmService, ApiError> {
    state
        .nats_dm_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalServerError("NATS DM service not available".into()))
}

/// Parse conv_id format `{social_account_id}_{remote_user_id}` and return both parts.
fn parse_conv_id(conv_id: &str) -> Result<(i32, &str), ApiError> {
    let parts: Vec<&str> = conv_id.splitn(2, '_').collect();
    if parts.len() != 2 {
        return Err(ApiError::BadRequest("Invalid conv_id format, expected {account_id}_{remote_user}".into()));
    }
    let social_account_id: i32 = parts[0]
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid social_account_id in conv_id".into()))?;
    Ok((social_account_id, parts[1]))
}

/// Verify `conv_id` belongs to the authenticated user.
///
/// For real social accounts (id >= 0): checks database ownership.
/// For auto-generated accounts (id < 0): checks NATS KV existence under user prefix.
async fn verify_conv_ownership(
    state: &UserState,
    conv_id: &str,
    user_id: i32,
) -> Result<i32, ApiError> {
    let (social_account_id, _) = parse_conv_id(conv_id)?;
    if social_account_id < 0 {
        // Auto-generated account (from executor): verify via NATS KV
        let nats_dm = require_nats_dm(state)?;
        nats_dm.verify_conv_exists(user_id, conv_id).await?;
    } else {
        // Real DB account: verify via database
        state
            .social_account_service
            .get_account_by_id(social_account_id, user_id)
            .await?;
    }
    Ok(social_account_id)
}

/// Verify the device_id belongs to at least one social account owned by the user.
async fn verify_device_ownership(
    state: &UserState,
    device_id: &str,
    user_id: i32,
) -> Result<(), ApiError> {
    let req = crate::dto::social_account_dto::AccountListRequest {
        device_id: Some(device_id.to_string()),
        page: 1,
        page_size: 1,
        group_id: None,
        username: None,
        platform_id: None,
        status: None,
    };
    let result = state.social_account_service.list_accounts(user_id, req).await?;
    if result.total == 0 {
        return Err(ApiError::NotFound(
            format!("No accounts bound to device {device_id}"),
        ));
    }
    Ok(())
}

/// GET /dm/conversations
pub async fn list_conversations(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Query(query): Query<DmConversationsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = require_nats_dm(&state)?;
    let response = nats_dm.list_conversations(user.id, query).await?;
    Ok(api_ok!(response))
}

/// GET /dm/conversations/:conv_id/messages
pub async fn get_messages(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(conv_id): Path<String>,
    Query(query): Query<DmMessagesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = require_nats_dm(&state)?;
    verify_conv_ownership(&state, &conv_id, user.id).await?;
    let response = nats_dm.get_messages(&conv_id, query).await?;
    Ok(api_ok!(response))
}

/// POST /dm/conversations/:conv_id/reply
pub async fn send_reply(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(conv_id): Path<String>,
    Json(req): Json<DmReplyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = require_nats_dm(&state)?;
    req.validate()?;

    let (social_account_id, remote_username) = parse_conv_id(&conv_id)?;

    let (device_id, platform_id, profile_name);
    if social_account_id < 0 {
        // Auto-generated account: get info from NATS KV conversation metadata
        let conv = nats_dm.get_conversation_meta(user.id, &conv_id).await?;
        device_id = conv.device_id;
        platform_id = conv.platform_id;
        profile_name = conv.my_profile_name;
    } else {
        // Real DB account: get info from database
        let account = state
            .social_account_service
            .get_account_by_id(social_account_id, user.id)
            .await?;
        device_id = account
            .device_id
            .unwrap_or_default();
        platform_id = account.platform_id;
        profile_name = account
            .profile_name
            .unwrap_or_default();
    }

    if device_id.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "Missing device_id for social account".into(),
        ));
    }
    if profile_name.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "Missing profile_name for social account".into(),
        ));
    }

    let response = nats_dm
        .send_reply(
            &conv_id,
            &device_id,
            social_account_id,
            platform_id,
            &profile_name,
            remote_username,
            &req.content,
            &req.content_type,
        )
        .await?;
    Ok(api_ok!(response))
}

/// POST /dm/conversations/:conv_id/read
pub async fn mark_read(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(conv_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = require_nats_dm(&state)?;
    verify_conv_ownership(&state, &conv_id, user.id).await?;
    nats_dm.mark_read(user.id, &conv_id).await?;
    Ok(api_ok!(msg: "Marked as read", "已标记为已读"))
}

/// PUT /dm/conversations/:conv_id/settings
pub async fn update_settings(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(conv_id): Path<String>,
    Json(req): Json<DmSettingsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = require_nats_dm(&state)?;
    req.validate()?;
    verify_conv_ownership(&state, &conv_id, user.id).await?;
    nats_dm.update_settings(user.id, &conv_id, req).await?;
    Ok(api_ok!(msg: "Settings updated", "设置已更新"))
}

/// GET /dm/stats
pub async fn get_stats(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = require_nats_dm(&state)?;
    let stats = nats_dm.get_stats(user.id).await?;
    Ok(api_ok!(stats))
}

/// GET /dm/monitor-config/:device_id
pub async fn get_monitor_config(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(device_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = require_nats_dm(&state)?;
    verify_device_ownership(&state, &device_id, user.id).await?;
    let config = nats_dm.get_monitor_config(&device_id).await?;
    Ok(api_ok!(config))
}

/// PUT /dm/monitor-config/:device_id
pub async fn update_monitor_config(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(device_id): Path<String>,
    Json(req): Json<DmMonitorConfigUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = require_nats_dm(&state)?;
    verify_device_ownership(&state, &device_id, user.id).await?;
    let config = nats_dm.update_monitor_config(&device_id, req).await?;
    Ok(api_ok!(config))
}

/// GET /dm/nats-token
pub async fn get_nats_token(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
) -> Result<impl IntoResponse, ApiError> {
    let nats_dm = require_nats_dm(&state)?;
    let response = nats_dm.generate_nats_token(user.id).await?;
    Ok(api_ok!(response))
}
