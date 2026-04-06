use crate::api_ok;
use crate::error::api_error::ApiError;
use crate::service::config_service::ConfigService;
use crate::service::platform_service::PlatformService;
use crate::service::video_capabilities::{build_video_model_capabilities, preferred_video_model};
use axum::{
    extract::{Path, Query},
    response::IntoResponse,
    Extension,
};
use glance_mind_db::entity::ai_model::AiModel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct AiModelResponseDto {
    #[serde(flatten)]
    pub model: AiModel,
    pub is_default: bool,
    pub capabilities: Option<crate::dto::video_dto::VideoModelCapabilitiesDto>,
}

pub async fn get_platforms(
    Extension(platform_service): Extension<PlatformService>,
) -> Result<impl IntoResponse, ApiError> {
    let platforms = platform_service
        .get_all_platforms()
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
    Ok(api_ok!(platforms))
}

pub async fn get_regions_by_platform(
    Path(platform_id): Path<i32>,
    Extension(platform_service): Extension<PlatformService>,
) -> Result<impl IntoResponse, ApiError> {
    let regions = platform_service
        .get_regions_by_platform(platform_id)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
    Ok(api_ok!(regions))
}

#[derive(Debug, Deserialize)]
pub struct GetAiModelsQuery {
    pub model_type: Option<String>,
}

pub async fn get_ai_models(
    Query(query): Query<GetAiModelsQuery>,
    Extension(config_service): Extension<ConfigService>,
) -> Result<impl IntoResponse, ApiError> {
    let models = if let Some(model_type) = query.model_type {
        config_service
            .get_ai_models_by_type(&model_type)
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
    } else {
        config_service
            .get_active_ai_models()
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
    };

    let preferred_video_model_id = preferred_video_model(
        &models
            .iter()
            .filter(|model| model.model_type == "video")
            .cloned()
            .collect::<Vec<_>>(),
    )
    .map(|model| model.id);

    let payload: Vec<AiModelResponseDto> = models
        .into_iter()
        .map(|model| {
            let is_video = model.model_type == "video";
            let is_default = preferred_video_model_id == Some(model.id);
            let capabilities = if is_video {
                Some(build_video_model_capabilities(&model))
            } else {
                None
            };

            AiModelResponseDto {
                model,
                is_default,
                capabilities,
            }
        })
        .collect();

    Ok(api_ok!(payload))
}

pub async fn get_pricing(
    Extension(config_service): Extension<ConfigService>,
) -> Result<impl IntoResponse, ApiError> {
    let pricing = config_service
        .get_pricing_rules()
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
    Ok(api_ok!(pricing))
}
