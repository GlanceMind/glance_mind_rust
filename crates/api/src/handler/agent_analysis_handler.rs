use crate::api_ok;
use crate::dto::agent_analysis_dto::AgentAnalysisRequest;
use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use axum::{extract::State, response::IntoResponse, Extension, Json};
use glance_mind_db::entity::user::User;
use validator::Validate;

/// Agent comment analysis endpoint
///
/// Returns unified ApiResponse format
pub async fn analyze_comments(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Json(payload): Json<AgentAnalysisRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate request parameters
    payload
        .validate()
        .map_err(|e| ApiError::ValidationError(e.to_string()))?;

    // Call service for AI analysis
    let response = state
        .agent_analysis_service
        .analyze_comments(user.id, payload)
        .await?;

    // Return unified format success response
    Ok(api_ok!(response))
}
