use crate::dto::template_dto::{
    AssignReusableTemplateDto, ReusableTemplateCreateDto, ReusableTemplateUpdateDto,
    TemplateCreateDto, TemplateUpdateDto,
};
use crate::error::api_error::ApiError;
use crate::error::request_error::ValidatedRequest;
use crate::state::user_state::UserState;
use crate::{api_result, response::ApiResult};
use axum::extract::{Extension, Path, Query, State};
use glance_mind_db::entity::user::User;

pub async fn list_templates(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(campaign_id): Path<i32>,
    Query(req): Query<crate::dto::common::PageRequest>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let response = state
        .template_service
        .get_templates(user.id, campaign_id, req)
        .await?;
    Ok(api_result!(response))
}

pub async fn list_all_templates(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Query(req): Query<crate::dto::common::PageRequest>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let response = state
        .template_service
        .get_all_templates(user.id, req)
        .await?;
    Ok(api_result!(response))
}

pub async fn get_template(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(template_id): Path<i32>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let template = state
        .template_service
        .get_template(user.id, template_id)
        .await?;
    Ok(api_result!(template))
}

pub async fn create_template(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(campaign_id): Path<i32>,
    ValidatedRequest(payload): ValidatedRequest<TemplateCreateDto>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let template = state
        .template_service
        .create_template(user.id, campaign_id, payload)
        .await?;
    Ok(api_result!(
        template,
        "Template created successfully",
        "Template created successfully"
    ))
}

pub async fn update_template(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(template_id): Path<i32>,
    ValidatedRequest(payload): ValidatedRequest<TemplateUpdateDto>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let template = state
        .template_service
        .update_template(user.id, template_id, payload)
        .await?;
    Ok(api_result!(
        template,
        "Template updated successfully",
        "Template updated successfully"
    ))
}

pub async fn delete_template(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(template_id): Path<i32>,
) -> Result<ApiResult<()>, ApiError> {
    state
        .template_service
        .delete_template(user.id, template_id)
        .await?;
    Ok(api_result!(msg: "Template deleted successfully", "Template deleted successfully"))
}

pub async fn auto_generate(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let product_description = payload
        .get("product_description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let target_audience = payload
        .get("target_audience")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let style_preference = payload
        .get("style_preference")
        .and_then(|v| v.as_str())
        .unwrap_or("friendly")
        .to_string();
    let count = payload.get("count").and_then(|v| v.as_i64()).unwrap_or(1) as i32;

    let templates = state
        .template_service
        .auto_generate_templates(
            user.id,
            &product_description,
            &target_audience,
            &style_preference,
            count.min(10),
        )
        .await?;

    Ok(api_result!(serde_json::json!({
        "templates": templates,
        "count": templates.len()
    })))
}

pub async fn list_reusable_templates(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Query(req): Query<crate::dto::common::PageRequest>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let response = state
        .template_service
        .list_reusable_templates(user.id, req)
        .await?;
    Ok(api_result!(response))
}

pub async fn get_reusable_template(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(template_id): Path<i32>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let template = state
        .template_service
        .get_reusable_template(user.id, template_id)
        .await?;
    Ok(api_result!(template))
}

pub async fn create_reusable_template(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    ValidatedRequest(payload): ValidatedRequest<ReusableTemplateCreateDto>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let template = state
        .template_service
        .create_reusable_template(user.id, payload)
        .await?;
    Ok(api_result!(
        template,
        "Reusable template created successfully",
        "Reusable template created successfully"
    ))
}

pub async fn update_reusable_template(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(template_id): Path<i32>,
    ValidatedRequest(payload): ValidatedRequest<ReusableTemplateUpdateDto>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let template = state
        .template_service
        .update_reusable_template(user.id, template_id, payload)
        .await?;
    Ok(api_result!(
        template,
        "Reusable template updated successfully",
        "Reusable template updated successfully"
    ))
}

pub async fn delete_reusable_template(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(template_id): Path<i32>,
) -> Result<ApiResult<()>, ApiError> {
    state
        .template_service
        .delete_reusable_template(user.id, template_id)
        .await?;
    Ok(
        api_result!(msg: "Reusable template deleted successfully", "Reusable template deleted successfully"),
    )
}

pub async fn assign_reusable_template(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(template_id): Path<i32>,
    ValidatedRequest(payload): ValidatedRequest<AssignReusableTemplateDto>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let template = state
        .template_service
        .assign_reusable_template(user.id, template_id, payload)
        .await?;
    Ok(api_result!(
        template,
        "Reusable template assigned successfully",
        "Reusable template assigned successfully"
    ))
}
