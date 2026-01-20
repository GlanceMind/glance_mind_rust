use crate::api_ok;
use crate::dto::user_dto::UserRegisterDto;
use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use axum::{extract::State, response::IntoResponse, Json};

/// User registration endpoint
///
/// Returns unified ApiResponse format
pub async fn register(
    State(state): State<UserState>,
    Json(payload): Json<UserRegisterDto>,
) -> Result<impl IntoResponse, ApiError> {
    // Create user and generate authentication info
    let user = state.user_service.create_user(payload, &state.db).await?;

    // Return unified format response
    Ok(api_ok!(
        user,
        "User registered successfully",
        "User registered successfully"
    ))
}
