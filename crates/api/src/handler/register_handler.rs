use crate::api_ok;
use crate::dto::user_dto::UserRegisterDto;
use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};

/// User registration endpoint
///
/// Returns unified ApiResponse format
pub async fn register(
    State(state): State<UserState>,
    headers: HeaderMap,
    Json(payload): Json<UserRegisterDto>,
) -> Result<impl IntoResponse, ApiError> {
    // Capture notification fields before `payload` is moved into the service.
    let notify_ctx = state.feishu_client.clone().map(|tg| {
        let ip = headers
            .get("x-forwarded-for")
            .or_else(|| headers.get("x-real-ip"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        (
            tg,
            payload.username.clone(),
            payload.email.clone(),
            payload.phone.clone(),
            ip,
        )
    });

    // Create user and generate authentication info
    let user = state.user_service.create_user(payload, &state.db).await?;

    // Fire-and-forget internal Feishu notification (best-effort, post-success).
    if let Some((tg, username, email, phone, ip)) = notify_ctx {
        tg.notify(crate::service::feishu_client::format_user_registered(
            &username,
            &email,
            &phone,
            &ip,
            chrono::Utc::now(),
        ));
    }

    // Return unified format response
    Ok(api_ok!(
        user,
        "User registered successfully",
        "User registered successfully"
    ))
}
