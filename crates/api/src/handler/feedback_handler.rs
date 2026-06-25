//! Public contact / feedback form handler.
//!
//! Accepts `{ email, phone, description }` from any page (no auth required, so it
//! also works on public/landing pages) and pushes a Feishu notification to the
//! internal channel. Best-effort: a Feishu failure never fails the submission.

use crate::api_ok;
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::state::user_state::UserState;
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use serde::Deserialize;

/// Basic abuse guard: cap the stored/forwarded description length.
const MAX_DESCRIPTION_CHARS: usize = 2000;

#[derive(Debug, Deserialize)]
pub struct FeedbackDto {
    pub email: String,
    #[serde(default)]
    pub phone: String,
    pub description: String,
}

/// POST /feedback/submit — public; validates and pushes the feedback to Feishu.
pub async fn submit(
    State(state): State<UserState>,
    headers: HeaderMap,
    Json(payload): Json<FeedbackDto>,
) -> Result<impl IntoResponse, ApiError> {
    let email = payload.email.trim();
    let phone = payload.phone.trim();
    let description = payload.description.trim();

    if email.is_empty() || !email.contains('@') {
        return Err(ApiError::BusinessError(BusinessError::ValidationFailed(
            "A valid email is required".to_string(),
        )));
    }
    if description.is_empty() {
        return Err(ApiError::BusinessError(BusinessError::ValidationFailed(
            "Description is required".to_string(),
        )));
    }

    let description: String = description.chars().take(MAX_DESCRIPTION_CHARS).collect();

    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Fire-and-forget Feishu notification (best-effort; never blocks the response).
    if let Some(tg) = state.feishu_client.clone() {
        tg.notify(crate::service::feishu_client::format_feedback(
            email,
            phone,
            &description,
            &ip,
            chrono::Utc::now(),
        ));
    }

    Ok(api_ok!(
        serde_json::json!({ "submitted": true }),
        "Feedback submitted",
        "反馈已提交"
    ))
}
