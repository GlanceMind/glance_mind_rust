use crate::dto::material_dto::{
    CreateMaterialRequest, MaterialListQuery, UpdateMaterialRequest,
};
use crate::error::api_error::ApiError;
use crate::error::request_error::ValidatedRequest;
use crate::response::api_result::ApiResult;
use crate::state::user_state::UserState;
use axum::extract::{Extension, Path, Query, State};
use glance_mind_db::entity::user::User;

/// List user materials
/// GET /api/v1/materials
pub async fn list_materials(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Query(query): Query<MaterialListQuery>,
) -> Result<ApiResult<crate::dto::material_dto::MaterialListResponse>, ApiError> {
    let response = state
        .material_service
        .list_materials(user.id, query)
        .await?;
    Ok(ApiResult::ok(response))
}

/// Get material by ID
/// GET /api/v1/materials/:id
pub async fn get_material(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(id): Path<i32>,
) -> Result<ApiResult<crate::dto::material_dto::MaterialDetail>, ApiError> {
    let material = state
        .material_service
        .get_material(id, user.id)
        .await?;
    Ok(ApiResult::ok(material))
}

/// Create material (user upload)
/// POST /api/v1/materials
pub async fn create_material(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    ValidatedRequest(request): ValidatedRequest<CreateMaterialRequest>,
) -> Result<ApiResult<crate::dto::material_dto::MaterialDetail>, ApiError> {
    let material = state
        .material_service
        .create_material(user.id, request)
        .await?;
    Ok(ApiResult::ok(material))
}

/// Update material
/// PUT /api/v1/materials/:id
pub async fn update_material(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(id): Path<i32>,
    ValidatedRequest(request): ValidatedRequest<UpdateMaterialRequest>,
) -> Result<ApiResult<crate::dto::material_dto::MaterialDetail>, ApiError> {
    let material = state
        .material_service
        .update_material(id, user.id, request)
        .await?;
    Ok(ApiResult::ok(material))
}

/// Delete material
/// DELETE /api/v1/materials/:id
pub async fn delete_material(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(id): Path<i32>,
) -> Result<ApiResult<()>, ApiError> {
    state
        .material_service
        .delete_material(id, user.id)
        .await?;
    Ok(ApiResult::ok(()))
}

/// Collect tags from video_cases
/// GET /api/v1/material-tags
pub async fn list_tags(
    State(state): State<UserState>,
) -> Result<ApiResult<crate::dto::material_dto::MaterialTagsResponse>, ApiError> {
    let response = state.material_service.collect_tags().await?;
    Ok(ApiResult::ok(response))
}

/// Favorite material from video_case
/// POST /api/v1/video-cases/:task_no/favorite
pub async fn favorite_from_video_case(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(task_no): Path<String>,
) -> Result<ApiResult<crate::dto::material_dto::MaterialDetail>, ApiError> {
    let material = state
        .material_service
        .favorite_from_video_case(user.id, &task_no)
        .await?;
    Ok(ApiResult::ok(material))
}
