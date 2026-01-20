use crate::api_ok;
use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};

pub async fn list_campaign_tasks(
    State(state): State<UserState>,
    Path(campaign_id): Path<i32>,
    Query(req): Query<crate::dto::common::PageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .crawler_service
        .list_campaign_tasks(campaign_id, req)
        .await?;
    Ok(api_ok!(response))
}

pub async fn list_task_results(
    State(state): State<UserState>,
    Path(task_id): Path<i32>,
    Query(req): Query<crate::dto::common::PageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .crawler_service
        .list_task_results(task_id, req)
        .await?;
    Ok(api_ok!(response))
}

pub async fn list_campaign_results(
    State(state): State<UserState>,
    Path(campaign_id): Path<i32>,
    Query(req): Query<crate::dto::common::PageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .crawler_service
        .list_campaign_results(campaign_id, req)
        .await?;
    Ok(api_ok!(response))
}
