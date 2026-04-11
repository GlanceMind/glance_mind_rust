use crate::dto::material_folder_dto::{CreateFolderRequest, FolderListQuery, UpdateFolderRequest};
use crate::error::api_error::ApiError;
use crate::error::request_error::ValidatedRequest;
use crate::response::api_result::ApiResult;
use crate::state::user_state::UserState;
use axum::extract::{Extension, Path, Query, State};
use glance_mind_db::entity::user::User;

pub async fn list_folders(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Query(query): Query<FolderListQuery>,
) -> Result<ApiResult<crate::dto::material_folder_dto::FolderListResponse>, ApiError> {
    let response = state
        .material_folder_service
        .list_folders(user.id, query.parent_id)
        .await?;
    Ok(ApiResult::ok(response))
}

pub async fn create_folder(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    ValidatedRequest(request): ValidatedRequest<CreateFolderRequest>,
) -> Result<ApiResult<crate::dto::material_folder_dto::FolderDetail>, ApiError> {
    let folder = state
        .material_folder_service
        .create_folder(user.id, request)
        .await?;
    Ok(ApiResult::ok(folder))
}

pub async fn update_folder(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(id): Path<i32>,
    ValidatedRequest(request): ValidatedRequest<UpdateFolderRequest>,
) -> Result<ApiResult<crate::dto::material_folder_dto::FolderDetail>, ApiError> {
    let folder = state
        .material_folder_service
        .update_folder(id, user.id, request)
        .await?;
    Ok(ApiResult::ok(folder))
}

pub async fn delete_folder(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(id): Path<i32>,
) -> Result<ApiResult<()>, ApiError> {
    state
        .material_folder_service
        .delete_folder(id, user.id)
        .await?;
    Ok(ApiResult::ok(()))
}

pub async fn get_folder_tree(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
) -> Result<ApiResult<crate::dto::material_folder_dto::FolderTreeResponse>, ApiError> {
    let tree = state
        .material_folder_service
        .get_folder_tree(user.id)
        .await?;
    Ok(ApiResult::ok(tree))
}
