use crate::api_ok;
use crate::dto::dm_auto_reply_dto::{EscalateRequest, InternalReplyRequest, ReplyLogUpsertRequest};
use crate::error::api_error::ApiError;
use crate::middleware::internal_token::require_internal_token;
use crate::service::dm_auto_reply_service::DmAutoReplyService;
use crate::state::user_state::UserState;
use axum::{
    extract::{Extension, Path},
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use glance_mind_db::entity::user::User;
use std::sync::Arc;

pub fn routes() -> Router<UserState> {
    Router::new()
        .route("/dm/config/:social_account_id", get(get_config))
        .route("/dm/rate-limit/:campaign_id/:user_ref", get(get_rate_limit))
        .route("/dm/reply-log", post(upsert_reply_log))
        .route("/dm/reply", post(send_reply))
        .route("/dm/escalate", post(escalate))
        .layer(middleware::from_fn(require_internal_token))
}

pub fn admin_routes() -> Router<UserState> {
    Router::new().route(
        "/dm/conversations/:conv_id/clear-review",
        post(clear_review),
    )
}

fn service(state: &UserState) -> DmAutoReplyService {
    DmAutoReplyService::new(&Arc::clone(&state.db), state.nats_dm_service.clone())
}

async fn get_config(
    Extension(state): Extension<UserState>,
    Path(social_account_id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    let svc = service(&state);
    let resp = svc.get_config(social_account_id)?;
    Ok(api_ok!(resp))
}

async fn get_rate_limit(
    Extension(state): Extension<UserState>,
    Path((campaign_id, user_ref)): Path<(i32, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let svc = service(&state);
    let resp = svc.get_rate_limit(campaign_id, &user_ref)?;
    Ok(api_ok!(resp))
}

async fn upsert_reply_log(
    Extension(state): Extension<UserState>,
    Json(req): Json<ReplyLogUpsertRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let svc = service(&state);
    let resp = svc.upsert_reply_log(req)?;
    Ok(api_ok!(resp))
}

async fn send_reply(
    Extension(state): Extension<UserState>,
    Json(req): Json<InternalReplyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let svc = service(&state);
    let resp = svc
        .send_reply(&req.conv_id, &req.content, &req.inbound_msg_id)
        .await?;
    Ok(api_ok!(resp))
}

async fn escalate(
    Extension(state): Extension<UserState>,
    Json(req): Json<EscalateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let svc = service(&state);
    let resp = svc.escalate(&req.conv_id, &req.reason, &req.inbound_msg_id)?;
    Ok(api_ok!(resp))
}

async fn clear_review(
    Extension(state): Extension<UserState>,
    Extension(user): Extension<User>,
    Path(conv_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let svc = service(&state);
    // P2-5: `resolved_by` must come from the authenticated admin's JWT, not
    // from the request body, so the audit trail cannot be spoofed.
    let resp = svc.clear_review(&conv_id, user.id)?;
    Ok(api_ok!(resp))
}
