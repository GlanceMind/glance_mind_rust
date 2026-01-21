use crate::api_ok;
use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use axum::{extract::Query, response::IntoResponse, Extension};
use glance_mind_db::entity::user::User;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PerformanceQuery {
    pub days: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct RecentCampaignsQuery {
    pub limit: Option<i32>,
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

pub async fn get_recent_campaigns(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Query(query): Query<RecentCampaignsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(3);
    let campaigns = state
        .dashboard_service
        .get_recent_campaigns(user.id, limit)
        .await?;
    Ok(api_ok!(campaigns))
}
