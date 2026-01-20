use crate::api_ok;
use glance_mind_db::entity::user::User;
use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use axum::{extract::Query, response::IntoResponse, Extension};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PerformanceQuery {
    pub days: Option<i32>,
}

pub async fn get_overview_stats(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
) -> Result<impl IntoResponse, ApiError> {
    let stats = state.dashboard_service.get_overview_stats(user.id).await?;
    Ok(api_ok!(stats))
}

pub async fn get_performance_stats(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Query(query): Query<PerformanceQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let stats = state
        .dashboard_service
        .get_performance_stats(user.id, query.days)
        .await?;
    Ok(api_ok!(stats))
}
