use crate::api_result;
use crate::dto::campaign_dto::{CampaignCreateDto, CampaignStatusUpdateDto, CampaignUpdateDto};
use crate::error::api_error::ApiError;
use crate::response::ApiResult;
use crate::service::campaign_service::CampaignService;
use axum::{
    extract::{Path, Query},
    Extension, Json,
};
use glance_mind_db::entity::user::User;

pub async fn create_campaign(
    Extension(user): Extension<User>,
    Extension(campaign_service): Extension<CampaignService>,
    Json(dto): Json<CampaignCreateDto>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let campaign = campaign_service.create_campaign(user.id, dto).await?;
    Ok(api_result!(campaign))
}

pub async fn list_campaigns(
    Extension(user): Extension<User>,
    Extension(campaign_service): Extension<CampaignService>,
    Query(req): Query<crate::dto::common::PageRequest>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let response = campaign_service.list_campaigns(user.id, req).await?;
    Ok(api_result!(response))
}

pub async fn get_campaign(
    Extension(user): Extension<User>,
    Extension(campaign_service): Extension<CampaignService>,
    Path(id): Path<i32>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let campaign = campaign_service.get_campaign(id, user.id).await?;
    Ok(api_result!(campaign))
}

pub async fn update_campaign(
    Extension(user): Extension<User>,
    Extension(campaign_service): Extension<CampaignService>,
    Path(id): Path<i32>,
    Json(dto): Json<CampaignUpdateDto>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let campaign = campaign_service.update_campaign(id, user.id, dto).await?;
    Ok(api_result!(campaign))
}

pub async fn update_campaign_status(
    Extension(user): Extension<User>,
    Extension(campaign_service): Extension<CampaignService>,
    Path(id): Path<i32>,
    Json(dto): Json<CampaignStatusUpdateDto>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let campaign = campaign_service
        .update_status(id, user.id, dto.status)
        .await?;
    Ok(api_result!(campaign))
}

pub async fn get_campaign_logs(
    Extension(user): Extension<User>,
    Extension(campaign_service): Extension<CampaignService>,
    Path(id): Path<i32>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let logs = campaign_service.get_campaign_logs(id, user.id).await?;
    Ok(api_result!(logs))
}

pub async fn get_campaign_lead_metrics(
    Extension(user): Extension<User>,
    Extension(campaign_service): Extension<CampaignService>,
    Path(id): Path<i32>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let metrics = campaign_service.get_lead_metrics(id, user.id).await?;
    Ok(api_result!(metrics))
}
