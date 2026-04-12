use axum::{
    body::BoxBody,
    extract::{Multipart, Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use chrono::Utc;
use glance_mind_db::entity::user::User;
use serde_json::Value;
use uuid::Uuid;

use crate::dto::drama_dto::DramaChapterSceneAssetsRequest;
use crate::dto::drama_dto::*;
use crate::dto::oss_dto::UploadImageResponse;
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::response::api_result::ApiResult;
use crate::service::drama_billing::DramaBillingGuard;
use crate::service::drama_facade::{DramaFacade, FacadeError};
use crate::service::drama_private_assets_service::{
    ChapterSceneAssetsError, DramaPrivateAssetsService, ProjectResourcesError,
};
use crate::service::drama_project_meta_service::{DramaProjectMetaRow, DramaProjectMetaService};
use crate::service::drama_projection::{DramaProjectionService, ProjectionRow};
use crate::service::drama_worker_dispatcher::DramaWorkerDispatcher;
use crate::state::user_state::UserState;

fn map_facade_err(e: FacadeError) -> Response<BoxBody> {
    tracing::error!("Drama facade error: {}", e);
    (
        StatusCode::BAD_GATEWAY,
        Json(serde_json::json!({
            "code": 502,
            "msg": "Drama service temporarily unavailable",
            "msg_cn": "短剧服务暂时不可用",
        })),
    )
        .into_response()
}

fn gateway_response(status: u16, body: Value) -> Response<BoxBody> {
    let http_status = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    if http_status.is_success() {
        let wrapped = serde_json::json!({
            "code": 1000,
            "msg": "success",
            "msg_cn": "成功",
            "data": body,
        });
        (http_status, Json(wrapped)).into_response()
    } else {
        (http_status, Json(body)).into_response()
    }
}

#[allow(dead_code)]
fn canonical_reads_enabled() -> bool {
    matches!(
        std::env::var("DRAMA_CANONICAL_READS")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn projection_summary_json(row: &ProjectionRow) -> Value {
    serde_json::json!({
        "id": row.id,
        "project_id": row.project_id,
        "status": row.status,
        "title": row.title,
        "description": row.description,
        "content_type": row.content_type,
        "current_stage": row.current_stage,
        "progress_percent": row.progress_percent,
        "cost": {
            "estimated_cents": row.cost_reserve_cents,
            "consumed_cents": row.cost_consumed_cents,
            "frozen_points": 0
        },
        "created_at": row.created_at.and_utc().to_rfc3339(),
        "updated_at": row.updated_at.and_utc().to_rfc3339()
    })
}

fn projection_detail_json(row: &ProjectionRow) -> Value {
    serde_json::json!({
        "project_id": row.project_id,
        "title": row.title,
        "description": row.description,
        "status": row.status,
        "content_type": row.content_type,
        "platform": row.platform,
        "current_stage": row.current_stage,
        "pending_stage": row.pending_stage,
        "progress_percent": row.progress_percent,
        "run_id": row.run_id,
        "interaction_version": row.interaction_version,
        "error_message": row.error_message,
        "cost_summary": {
            "reserve_cents": row.cost_reserve_cents,
            "consumed_cents": row.cost_consumed_cents
        },
        "interaction": row.interaction_payload,
        "created_at": row.created_at.and_utc().to_rfc3339(),
        "updated_at": row.updated_at.and_utc().to_rfc3339(),
        "completed_at": row.completed_at.map(|v| v.and_utc().to_rfc3339())
    })
}

pub async fn preflight(
    Extension(user): Extension<User>,
    Extension(facade): Extension<DramaFacade>,
    Json(req): Json<DramaPreflightRequest>,
) -> impl IntoResponse {
    let body = serde_json::to_value(&req).unwrap_or_default();
    match facade.preflight(body, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e).into_response(),
    }
}

pub async fn create_project(
    Extension(user): Extension<User>,
    Extension(facade): Extension<DramaFacade>,
    Extension(billing): Extension<DramaBillingGuard>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(dispatcher): Extension<Option<DramaWorkerDispatcher>>,
    Json(req): Json<DramaProjectCreateRequest>,
) -> Response<BoxBody> {
    let estimated_cost: i64 = req
        .extra
        .get("budget_cents")
        .and_then(|v| v.as_i64())
        .unwrap_or(10000);

    match billing.check_balance(user.id, estimated_cost).await {
        Ok(check) if !check.sufficient => {
            return (
                StatusCode::PAYMENT_REQUIRED,
                Json(serde_json::json!({
                    "code": 402,
                    "msg": "Insufficient balance for this drama project",
                    "msg_cn": "余额不足，无法创建短剧项目",
                    "data": {
                        "available_points": check.balance_points,
                        "estimated_cost_points": check.estimated_cost_points,
                    }
                })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Billing guard error: {}", e);
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "code": 503,
                    "msg": "Billing service unavailable",
                    "msg_cn": "计费服务暂时不可用，请稍后重试",
                })),
            )
                .into_response();
        }
        _ => {}
    }

    if direct_worker_mode_enabled() {
        match create_project_direct_via_worker(
            dispatcher.as_ref(),
            &projection,
            &user,
            &req,
            estimated_cost,
        )
        .await
        {
            Ok(body) => return gateway_response(201, body),
            Err(err) => {
                tracing::error!("Direct worker create project failed: {}", err);
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({
                        "code": 503,
                        "msg": "Drama worker unavailable",
                        "msg_cn": "短剧 worker 当前不可用",
                        "detail": err,
                    })),
                )
                    .into_response();
            }
        }
    }

    let body = serde_json::to_value(&req).unwrap_or_default();
    match facade.create_project(body, &user).await {
        Ok((status, body)) => {
            if let Some(project_id) = body.get("project_id").and_then(|v| v.as_str()) {
                let content_type = req.extra.get("content_type").and_then(|v| v.as_str());
                let platform = req.extra.get("platform").and_then(|v| v.as_str());
                if let Err(err) = projection.create_projection(
                    project_id,
                    user.id,
                    &req.title,
                    &req.description,
                    content_type,
                    platform,
                    estimated_cost,
                ) {
                    tracing::error!("Drama projection bootstrap failed: {}", err);
                }
            }
            if let Err(err) = maybe_shadow_dispatch_worker_bootstrap(
                dispatcher.as_ref(),
                &projection,
                &user,
                &req,
                &body,
                estimated_cost,
            )
            .await
            {
                tracing::error!("Drama worker shadow bootstrap failed: {}", err);
            }
            gateway_response(status, body)
        }
        Err(e) => map_facade_err(e),
    }
}

fn worker_shadow_dispatch_enabled() -> bool {
    matches!(
        std::env::var("DRAMA_WORKER_SHADOW_DISPATCH")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn direct_worker_mode_enabled() -> bool {
    matches!(
        std::env::var("DRAMA_DIRECT_WORKER_MODE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn build_initial_worker_request_payload(
    req: &DramaProjectCreateRequest,
    project_id: &str,
) -> serde_json::Value {
    let content_type = req.extra.get("content_type").and_then(|v| v.as_str());
    let platform = req.extra.get("platform").and_then(|v| v.as_str());
    serde_json::json!({
        "project_id": project_id,
        "title": req.title,
        "description": req.description,
        "content_type": content_type,
        "platform": platform,
        "extra": req.extra,
    })
}

async fn dispatch_initial_outline_task(
    dispatcher: &DramaWorkerDispatcher,
    projection: &DramaProjectionService,
    user: &User,
    req: &DramaProjectCreateRequest,
    project_id: &str,
    reserve_cents: i64,
    interaction_version: i64,
) -> Result<(i64, DramaWorkerTaskEnvelope), String> {
    let content_type = req.extra.get("content_type").and_then(|v| v.as_str());
    let platform = req.extra.get("platform").and_then(|v| v.as_str());

    projection.create_projection(
        project_id,
        user.id,
        &req.title,
        &req.description,
        content_type,
        platform,
        reserve_cents,
    )?;

    let mut request_payload = build_initial_worker_request_payload(req, project_id);
    request_payload["sequence_base"] = serde_json::json!(1);
    let job_id = projection.create_job(
        project_id,
        Some("s01_strategy"),
        "generate_outline",
        Some(&format!("{}:generate_outline", project_id)),
        &request_payload,
    )?;

    let envelope = DramaWorkerTaskEnvelope {
        task_id: Uuid::new_v4().to_string(),
        project_id: project_id.to_string(),
        task_type: DramaWorkerTaskType::GenerateOutline,
        stage_code: Some("s01_strategy".to_string()),
        job_id: Some(job_id),
        interaction_version: Some(interaction_version),
        target: DramaWorkerTaskTarget::default(),
        payload: request_payload,
        created_at: Utc::now(),
    };

    dispatcher.enqueue(&envelope).await?;
    Ok((job_id, envelope))
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_followup_task(
    dispatcher: &DramaWorkerDispatcher,
    projection: &DramaProjectionService,
    row: &ProjectionRow,
    project_id: &str,
    task_type: DramaWorkerTaskType,
    stage_code: &str,
    job_type: &str,
    interaction_version: i64,
    payload: Value,
) -> Result<(i64, DramaWorkerTaskEnvelope), String> {
    let mut payload = payload;
    payload["sequence_base"] = serde_json::json!(row.last_event_sequence + 1);

    let job_id = projection.create_job(
        project_id,
        Some(stage_code),
        job_type,
        Some(&format!(
            "{}:{}:v{}",
            project_id, job_type, interaction_version
        )),
        &payload,
    )?;

    let envelope = DramaWorkerTaskEnvelope {
        task_id: Uuid::new_v4().to_string(),
        project_id: project_id.to_string(),
        task_type,
        stage_code: Some(stage_code.to_string()),
        job_id: Some(job_id),
        interaction_version: Some(interaction_version),
        target: DramaWorkerTaskTarget::default(),
        payload,
        created_at: Utc::now(),
    };

    dispatcher.enqueue(&envelope).await?;
    Ok((job_id, envelope))
}

async fn maybe_shadow_dispatch_worker_bootstrap(
    dispatcher: Option<&DramaWorkerDispatcher>,
    projection: &DramaProjectionService,
    user: &User,
    req: &DramaProjectCreateRequest,
    response_body: &Value,
    estimated_cost: i64,
) -> Result<(), String> {
    if !worker_shadow_dispatch_enabled() {
        return Ok(());
    }

    let dispatcher = match dispatcher {
        Some(dispatcher) => dispatcher,
        None => return Err("dispatcher unavailable".to_string()),
    };

    let project_id = response_body
        .get("project_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "project_id missing from upstream create response".to_string())?;

    let reserve_cents = req
        .extra
        .get("budget_cents")
        .and_then(|v| v.as_i64())
        .unwrap_or(estimated_cost);
    let interaction_version = response_body
        .get("interaction_version")
        .and_then(|v| v.as_i64())
        .or(Some(1));
    let (job_id, envelope) = dispatch_initial_outline_task(
        dispatcher,
        projection,
        user,
        req,
        project_id,
        reserve_cents,
        interaction_version.unwrap_or(1),
    )
    .await?;
    tracing::info!(
        project_id = project_id,
        job_id = job_id,
        task_id = %envelope.task_id,
        "Shadow-dispatched initial drama worker task"
    );
    Ok(())
}

async fn create_project_direct_via_worker(
    dispatcher: Option<&DramaWorkerDispatcher>,
    projection: &DramaProjectionService,
    user: &User,
    req: &DramaProjectCreateRequest,
    estimated_cost: i64,
) -> Result<Value, String> {
    let dispatcher = dispatcher.ok_or_else(|| "dispatcher unavailable".to_string())?;
    let project_id = Uuid::new_v4().to_string();
    let reserve_cents = req
        .extra
        .get("budget_cents")
        .and_then(|v| v.as_i64())
        .unwrap_or(estimated_cost);

    let (_job_id, envelope) = dispatch_initial_outline_task(
        dispatcher,
        projection,
        user,
        req,
        &project_id,
        reserve_cents,
        1,
    )
    .await?;

    tracing::info!(
        project_id = %project_id,
        task_id = %envelope.task_id,
        "Created direct worker-mode drama project"
    );

    Ok(serde_json::json!({
        "project_id": project_id,
        "status": "pending",
        "interaction_version": 1,
        "reserve_cents": reserve_cents
    }))
}

#[allow(clippy::result_large_err)]
fn check_projection_access(
    row: Option<ProjectionRow>,
    user: &User,
) -> Result<ProjectionRow, Response<BoxBody>> {
    match row {
        Some(row) if row.user_id == user.id => Ok(row),
        Some(_) => Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "code": 403,
                "msg": "forbidden",
                "msg_cn": "无权访问该短剧项目"
            })),
        )
            .into_response()),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "code": 404,
                "msg": "not found",
                "msg_cn": "短剧项目不存在"
            })),
        )
            .into_response()),
    }
}

fn stale_interaction_response() -> Response<BoxBody> {
    (
        StatusCode::CONFLICT,
        Json(serde_json::json!({
            "code": 409,
            "msg": "stale interaction version",
            "msg_cn": "交互版本已过期"
        })),
    )
        .into_response()
}

fn internal_error_response(msg: &str, msg_cn: &str) -> Response<BoxBody> {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "code": 500,
            "msg": msg,
            "msg_cn": msg_cn,
        })),
    )
        .into_response()
}

fn not_found_response(msg: &str, msg_cn: &str) -> Response<BoxBody> {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "code": 404,
            "msg": msg,
            "msg_cn": msg_cn,
        })),
    )
        .into_response()
}

fn forbidden_response(msg: &str, msg_cn: &str) -> Response<BoxBody> {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({
            "code": 403,
            "msg": msg,
            "msg_cn": msg_cn,
        })),
    )
        .into_response()
}

fn bad_request_response(msg: &str, msg_cn: &str) -> Response<BoxBody> {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "code": 400,
            "msg": msg,
            "msg_cn": msg_cn,
        })),
    )
        .into_response()
}

fn project_resources_not_ready_response() -> Response<BoxBody> {
    not_found_response(
        "drama project resources are not ready yet",
        "短剧项目资源暂未就绪，请稍后再试",
    )
}

fn private_assets_service(user_state: &UserState) -> DramaPrivateAssetsService {
    DramaPrivateAssetsService::new(&user_state.db)
}

fn project_meta_response_from_row(row: DramaProjectMetaRow) -> DramaProjectMetaResponse {
    DramaProjectMetaResponse {
        project_id: row.project_id,
        characters: row.characters.as_array().cloned().unwrap_or_default(),
        style_references: row.style_references.as_array().cloned().unwrap_or_default(),
        text_materials: row.text_materials.as_array().cloned().unwrap_or_default(),
        visual_settings: if row.visual_settings.is_object() {
            row.visual_settings
        } else {
            serde_json::json!({})
        },
        updated_at: row.updated_at.and_utc().to_rfc3339(),
    }
}

pub async fn get_project(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(facade): Extension<DramaFacade>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match projection.get_projection(&project_id) {
        Ok(Some(row)) if row.user_id == user.id => {
            return gateway_response(200, projection_detail_json(&row)).into_response();
        }
        Ok(Some(_)) => {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "code": 403,
                    "msg": "forbidden",
                    "msg_cn": "无权访问该短剧项目"
                })),
            )
                .into_response();
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("Projection read failed, falling through to facade: {}", e);
        }
    }
    match facade.get_project(&project_id, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e).into_response(),
    }
}

pub async fn get_project_meta(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(meta_service): Extension<DramaProjectMetaService>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    let projection_row = match projection.get_projection(&project_id) {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Projection read failed: {}", e);
            return internal_error_response("failed to read drama project", "读取短剧项目失败");
        }
    };
    if let Err(resp) = check_projection_access(projection_row, &user) {
        return resp;
    }

    let meta_row = match meta_service.get(&project_id, i64::from(user.id)) {
        Ok(Some(row)) => row,
        Ok(None) => DramaProjectMetaService::empty(&project_id, i64::from(user.id)),
        Err(e) => {
            tracing::error!("Drama project meta read failed: {}", e);
            return internal_error_response(
                "failed to read drama project meta",
                "读取短剧项目元数据失败",
            );
        }
    };

    gateway_response(
        200,
        serde_json::to_value(project_meta_response_from_row(meta_row)).unwrap_or_default(),
    )
}

pub async fn list_characters(
    Extension(user): Extension<User>,
    Extension(user_state): Extension<UserState>,
) -> impl IntoResponse {
    let service = private_assets_service(&user_state);
    match service.list_characters(i64::from(user.id)) {
        Ok(items) => gateway_response(
            200,
            serde_json::to_value(DramaPrivateCharacterListResponse { items }).unwrap_or_default(),
        ),
        Err(e) => {
            tracing::error!("List private characters failed: {}", e);
            internal_error_response("failed to list private characters", "读取私有角色列表失败")
        }
    }
}

pub async fn create_character(
    Extension(user): Extension<User>,
    Extension(user_state): Extension<UserState>,
    Json(req): Json<DramaPrivateCharacterRequest>,
) -> impl IntoResponse {
    let service = private_assets_service(&user_state);
    match service.create_character(i64::from(user.id), &req) {
        Ok(character) => gateway_response(200, serde_json::to_value(character).unwrap_or_default()),
        Err(e) => {
            tracing::error!("Create private character failed: {}", e);
            internal_error_response("failed to create private character", "创建私有角色失败")
        }
    }
}

pub async fn update_character(
    Extension(user): Extension<User>,
    Extension(user_state): Extension<UserState>,
    Path(id): Path<String>,
    Json(req): Json<DramaPrivateCharacterUpdateRequest>,
) -> impl IntoResponse {
    let service = private_assets_service(&user_state);
    match service.update_character(i64::from(user.id), &id, &req) {
        Ok(Some(character)) => {
            gateway_response(200, serde_json::to_value(character).unwrap_or_default())
        }
        Ok(None) => not_found_response("private character not found", "私有角色不存在"),
        Err(e) => {
            tracing::error!("Update private character failed: {}", e);
            internal_error_response("failed to update private character", "更新私有角色失败")
        }
    }
}

pub async fn delete_character(
    Extension(user): Extension<User>,
    Extension(user_state): Extension<UserState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let service = private_assets_service(&user_state);
    match service.delete_character(i64::from(user.id), &id) {
        Ok(true) => gateway_response(
            200,
            serde_json::json!({
                "id": id,
                "deleted": true,
            }),
        ),
        Ok(false) => not_found_response("private character not found", "私有角色不存在"),
        Err(e) => {
            tracing::error!("Delete private character failed: {}", e);
            internal_error_response("failed to delete private character", "删除私有角色失败")
        }
    }
}

pub async fn list_scene_assets(
    Extension(user): Extension<User>,
    Extension(user_state): Extension<UserState>,
) -> impl IntoResponse {
    let service = private_assets_service(&user_state);
    match service.list_scene_assets(i64::from(user.id)) {
        Ok(items) => gateway_response(
            200,
            serde_json::to_value(DramaPrivateSceneAssetListResponse { items }).unwrap_or_default(),
        ),
        Err(e) => {
            tracing::error!("List private scene assets failed: {}", e);
            internal_error_response(
                "failed to list private scene assets",
                "读取私有场景素材列表失败",
            )
        }
    }
}

pub async fn create_scene_asset(
    Extension(user): Extension<User>,
    Extension(user_state): Extension<UserState>,
    Json(req): Json<DramaPrivateSceneAssetRequest>,
) -> impl IntoResponse {
    let service = private_assets_service(&user_state);
    match service.create_scene_asset(i64::from(user.id), &req) {
        Ok(scene_asset) => {
            gateway_response(200, serde_json::to_value(scene_asset).unwrap_or_default())
        }
        Err(e) => {
            tracing::error!("Create private scene asset failed: {}", e);
            internal_error_response(
                "failed to create private scene asset",
                "创建私有场景素材失败",
            )
        }
    }
}

pub async fn update_scene_asset(
    Extension(user): Extension<User>,
    Extension(user_state): Extension<UserState>,
    Path(id): Path<String>,
    Json(req): Json<DramaPrivateSceneAssetUpdateRequest>,
) -> impl IntoResponse {
    let service = private_assets_service(&user_state);
    match service.update_scene_asset(i64::from(user.id), &id, &req) {
        Ok(Some(scene_asset)) => {
            gateway_response(200, serde_json::to_value(scene_asset).unwrap_or_default())
        }
        Ok(None) => not_found_response("private scene asset not found", "私有场景素材不存在"),
        Err(e) => {
            tracing::error!("Update private scene asset failed: {}", e);
            internal_error_response(
                "failed to update private scene asset",
                "更新私有场景素材失败",
            )
        }
    }
}

pub async fn delete_scene_asset(
    Extension(user): Extension<User>,
    Extension(user_state): Extension<UserState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let service = private_assets_service(&user_state);
    match service.delete_scene_asset(i64::from(user.id), &id) {
        Ok(true) => gateway_response(
            200,
            serde_json::json!({
                "id": id,
                "deleted": true,
            }),
        ),
        Ok(false) => not_found_response("private scene asset not found", "私有场景素材不存在"),
        Err(e) => {
            tracing::error!("Delete private scene asset failed: {}", e);
            internal_error_response(
                "failed to delete private scene asset",
                "删除私有场景素材失败",
            )
        }
    }
}

pub async fn list_style_assets(
    Extension(user): Extension<User>,
    Extension(user_state): Extension<UserState>,
) -> impl IntoResponse {
    let service = private_assets_service(&user_state);
    match service.list_style_assets(i64::from(user.id)) {
        Ok(items) => gateway_response(
            200,
            serde_json::to_value(DramaPrivateStyleAssetListResponse { items }).unwrap_or_default(),
        ),
        Err(e) => {
            tracing::error!("List private style assets failed: {}", e);
            internal_error_response(
                "failed to list private style assets",
                "读取私有样式素材列表失败",
            )
        }
    }
}

pub async fn create_style_asset(
    Extension(user): Extension<User>,
    Extension(user_state): Extension<UserState>,
    Json(req): Json<DramaPrivateStyleAssetRequest>,
) -> impl IntoResponse {
    let service = private_assets_service(&user_state);
    match service.create_style_asset(i64::from(user.id), &req) {
        Ok(style_asset) => {
            gateway_response(200, serde_json::to_value(style_asset).unwrap_or_default())
        }
        Err(e) => {
            tracing::error!("Create private style asset failed: {}", e);
            internal_error_response(
                "failed to create private style asset",
                "创建私有样式素材失败",
            )
        }
    }
}

pub async fn update_style_asset(
    Extension(user): Extension<User>,
    Extension(user_state): Extension<UserState>,
    Path(id): Path<String>,
    Json(req): Json<DramaPrivateStyleAssetUpdateRequest>,
) -> impl IntoResponse {
    let service = private_assets_service(&user_state);
    match service.update_style_asset(i64::from(user.id), &id, &req) {
        Ok(Some(style_asset)) => {
            gateway_response(200, serde_json::to_value(style_asset).unwrap_or_default())
        }
        Ok(None) => not_found_response("private style asset not found", "私有样式素材不存在"),
        Err(e) => {
            tracing::error!("Update private style asset failed: {}", e);
            internal_error_response(
                "failed to update private style asset",
                "更新私有样式素材失败",
            )
        }
    }
}

pub async fn delete_style_asset(
    Extension(user): Extension<User>,
    Extension(user_state): Extension<UserState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let service = private_assets_service(&user_state);
    match service.delete_style_asset(i64::from(user.id), &id) {
        Ok(true) => gateway_response(
            200,
            serde_json::json!({
                "id": id,
                "deleted": true,
            }),
        ),
        Ok(false) => not_found_response("private style asset not found", "私有样式素材不存在"),
        Err(e) => {
            tracing::error!("Delete private style asset failed: {}", e);
            internal_error_response(
                "failed to delete private style asset",
                "删除私有样式素材失败",
            )
        }
    }
}

pub async fn get_project_resources(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(user_state): Extension<UserState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match projection.get_projection(&project_id) {
        Ok(Some(row)) if row.user_id == user.id => {}
        Ok(Some(_)) => {
            return forbidden_response("forbidden", "无权访问该短剧项目资源");
        }
        Ok(None) => {
            return project_resources_not_ready_response();
        }
        Err(e) => {
            tracing::error!("Projection read failed: {}", e);
            return internal_error_response("failed to read drama project", "读取短剧项目失败");
        }
    }

    let service = private_assets_service(&user_state);
    match service.get_project_resources(&project_id, i64::from(user.id)) {
        Ok(resources) => gateway_response(200, serde_json::to_value(resources).unwrap_or_default()),
        Err(e) => {
            tracing::error!("Get project resources failed: {}", e);
            internal_error_response(
                "failed to read drama project resources",
                "读取短剧项目资源失败",
            )
        }
    }
}

pub async fn put_project_resources(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(user_state): Extension<UserState>,
    Path(project_id): Path<String>,
    Json(req): Json<DramaProjectResourcesRequest>,
) -> impl IntoResponse {
    match projection.get_projection(&project_id) {
        Ok(Some(row)) if row.user_id == user.id => {}
        Ok(Some(_)) => {
            return forbidden_response("forbidden", "无权修改该短剧项目资源");
        }
        Ok(None) => {
            return project_resources_not_ready_response();
        }
        Err(e) => {
            tracing::error!("Projection read failed: {}", e);
            return internal_error_response("failed to read drama project", "读取短剧项目失败");
        }
    }

    let service = private_assets_service(&user_state);
    match service.put_project_resources(&project_id, i64::from(user.id), &req) {
        Ok(resources) => gateway_response(200, serde_json::to_value(resources).unwrap_or_default()),
        Err(ProjectResourcesError::UnsupportedResourceFields) => bad_request_response(
            "scene_asset_ids are not supported at project level; bind scenes at chapter level",
            "scene_asset_ids 当前不支持项目级绑定，请在章节工作区完成场景绑定",
        ),
        Err(ProjectResourcesError::ForbiddenCharacterAssociation {
            invalid_character_ids,
        }) => (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "code": 403,
                "msg": "one or more character_ids do not belong to current user",
                "msg_cn": "存在不属于当前用户的 character_ids",
                "invalid_character_ids": invalid_character_ids,
            })),
        )
            .into_response(),
        Err(ProjectResourcesError::PrimaryStyleMustBeIncluded {
            primary_style_asset_id,
        }) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "code": 400,
                "msg": "primary_style_asset_id must be included in style_asset_ids",
                "msg_cn": "primary_style_asset_id 必须包含在 style_asset_ids 中",
                "primary_style_asset_id": primary_style_asset_id,
            })),
        )
            .into_response(),
        Err(ProjectResourcesError::ForbiddenStyleAssociation {
            invalid_style_asset_ids,
        }) => (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "code": 403,
                "msg": "one or more style_asset_ids do not belong to current user",
                "msg_cn": "存在不属于当前用户的 style_asset_ids",
                "invalid_style_asset_ids": invalid_style_asset_ids,
            })),
        )
            .into_response(),
        Err(ProjectResourcesError::Database(e)) => {
            tracing::error!("Put project resources failed: {}", e);
            internal_error_response(
                "failed to save drama project resources",
                "保存短剧项目资源失败",
            )
        }
    }
}

pub async fn get_chapter_scene_assets(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(user_state): Extension<UserState>,
    Path((project_id, chapter_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let projection_row = match projection.get_projection(&project_id) {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Projection read failed: {}", e);
            return internal_error_response("failed to read drama project", "读取短剧项目失败");
        }
    };
    if let Err(resp) = check_projection_access(projection_row, &user) {
        return resp;
    }

    let service = private_assets_service(&user_state);
    match service.get_chapter_scene_assets(&project_id, &chapter_id, i64::from(user.id)) {
        Ok(result) => gateway_response(200, serde_json::to_value(result).unwrap_or_default()),
        Err(ChapterSceneAssetsError::ForbiddenSceneAssetAssociation {
            invalid_scene_asset_ids,
        }) => (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "code": 403,
                "msg": "one or more scene_asset_ids do not belong to current user",
                "msg_cn": "存在不属于当前用户的 scene_asset_ids",
                "invalid_scene_asset_ids": invalid_scene_asset_ids,
            })),
        )
            .into_response(),
        Err(ChapterSceneAssetsError::Database(e)) => {
            tracing::error!("Get chapter scene assets failed: {}", e);
            internal_error_response(
                "failed to read chapter scene assets",
                "读取章节场景资产失败",
            )
        }
    }
}

pub async fn put_chapter_scene_assets(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(user_state): Extension<UserState>,
    Path((project_id, chapter_id)): Path<(String, String)>,
    Json(req): Json<DramaChapterSceneAssetsRequest>,
) -> impl IntoResponse {
    let projection_row = match projection.get_projection(&project_id) {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Projection read failed: {}", e);
            return internal_error_response("failed to read drama project", "读取短剧项目失败");
        }
    };
    if let Err(resp) = check_projection_access(projection_row, &user) {
        return resp;
    }

    let service = private_assets_service(&user_state);
    match service.put_chapter_scene_assets(
        &project_id,
        &chapter_id,
        i64::from(user.id),
        &req.scene_asset_ids,
    ) {
        Ok(result) => gateway_response(200, serde_json::to_value(result).unwrap_or_default()),
        Err(ChapterSceneAssetsError::ForbiddenSceneAssetAssociation {
            invalid_scene_asset_ids,
        }) => (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "code": 403,
                "msg": "one or more scene_asset_ids do not belong to current user",
                "msg_cn": "存在不属于当前用户的 scene_asset_ids",
                "invalid_scene_asset_ids": invalid_scene_asset_ids,
            })),
        )
            .into_response(),
        Err(ChapterSceneAssetsError::Database(e)) => {
            tracing::error!("Put chapter scene assets failed: {}", e);
            internal_error_response(
                "failed to save chapter scene assets",
                "保存章节场景资产失败",
            )
        }
    }
}

pub async fn upsert_project_meta(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(meta_service): Extension<DramaProjectMetaService>,
    Path(project_id): Path<String>,
    Json(req): Json<DramaProjectMetaRequest>,
) -> impl IntoResponse {
    let projection_row = match projection.get_projection(&project_id) {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Projection read failed: {}", e);
            return internal_error_response("failed to read drama project", "读取短剧项目失败");
        }
    };
    if let Err(resp) = check_projection_access(projection_row, &user) {
        return resp;
    }

    let visual_settings = if req.visual_settings.is_object() {
        req.visual_settings
    } else {
        serde_json::json!({})
    };
    let meta_row = match meta_service.upsert(
        &project_id,
        i64::from(user.id),
        &serde_json::json!(req.characters),
        &serde_json::json!(req.style_references),
        &serde_json::json!(req.text_materials),
        &visual_settings,
    ) {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Drama project meta upsert failed: {}", e);
            return internal_error_response(
                "failed to save drama project meta",
                "保存短剧项目元数据失败",
            );
        }
    };

    gateway_response(
        200,
        serde_json::to_value(project_meta_response_from_row(meta_row)).unwrap_or_default(),
    )
}

pub async fn list_projects(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(facade): Extension<DramaFacade>,
    Query(query): Query<DramaProjectListQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(20);

    if query.cursor.is_none() {
        match projection.list_projections(user.id, query.status.as_deref(), limit) {
            Ok(rows) => {
                let list: Vec<Value> = rows.iter().map(projection_summary_json).collect();
                return gateway_response(
                    200,
                    serde_json::json!({
                        "list": list,
                        "total": list.len(),
                        "page": 1,
                        "page_size": limit
                    }),
                )
                .into_response();
            }
            Err(e) => {
                tracing::error!("Projection list failed, falling through to facade: {}", e);
            }
        }
    }

    let mut pairs: Vec<(&str, String)> = Vec::new();
    if let Some(ref s) = query.status {
        pairs.push(("status", s.clone()));
    }
    if let Some(ref c) = query.cursor {
        pairs.push(("cursor", c.clone()));
    }
    if let Some(l) = query.limit {
        pairs.push(("limit", l.to_string()));
    }
    let refs: Vec<(&str, &str)> = pairs.iter().map(|(k, v)| (*k, v.as_str())).collect();
    match facade.list_projects(&refs, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => {
            tracing::error!("Drama facade list_projects failed: {}", e);
            gateway_response(
                200,
                serde_json::json!({
                    "list": [],
                    "total": 0,
                    "page": 1,
                    "page_size": limit
                }),
            )
            .into_response()
        }
    }
}

pub async fn cancel_project(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(facade): Extension<DramaFacade>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    if direct_worker_mode_enabled() {
        let row = match projection.get_projection(&project_id) {
            Ok(row) => row,
            Err(e) => {
                tracing::error!("Projection read failed: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "code": 500,
                        "msg": "projection read failed",
                        "msg_cn": "读取短剧项目失败",
                        "detail": e
                    })),
                )
                    .into_response();
            }
        };
        let row = match check_projection_access(row, &user) {
            Ok(row) => row,
            Err(resp) => return resp,
        };
        if let Err(e) = projection.cancel_project(&row.project_id) {
            tracing::error!("Direct worker cancel failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response();
        }
        return gateway_response(
            200,
            serde_json::json!({
                "project_id": row.project_id,
                "status": "cancelled",
                "accepted": true
            }),
        );
    }
    match facade.cancel_project(&project_id, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e).into_response(),
    }
}

pub async fn clarify(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(dispatcher): Extension<Option<DramaWorkerDispatcher>>,
    Extension(facade): Extension<DramaFacade>,
    Path(project_id): Path<String>,
    Json(req): Json<DramaClarifyRequest>,
) -> impl IntoResponse {
    if direct_worker_mode_enabled() {
        let row = match projection.get_projection(&project_id) {
            Ok(row) => row,
            Err(e) => {
                tracing::error!("Projection read failed: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e })),
                )
                    .into_response();
            }
        };
        let row = match check_projection_access(row, &user) {
            Ok(row) => row,
            Err(resp) => return resp,
        };
        if let Some(version) = req.interaction_version {
            if version != row.interaction_version as i64 {
                return stale_interaction_response();
            }
        }
        let dispatcher = match dispatcher.as_ref() {
            Some(dispatcher) => dispatcher,
            None => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({ "error": "dispatcher unavailable" })),
                )
                    .into_response();
            }
        };
        let stage = row.current_stage.as_deref().unwrap_or("s01_strategy");
        if let Err(e) = projection.mark_running(&row.project_id, Some(stage)) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response();
        }
        let payload = serde_json::json!({
            "project_id": row.project_id,
            "session_id": req.session_id,
            "answers": req.answers,
            "clarification_resolved": true,
            "source": "direct_worker_mode"
        });
        let task_type = match stage {
            "s03_script" => DramaWorkerTaskType::GenerateEpisodeScripts,
            "s04_visual" => DramaWorkerTaskType::GenerateStoryboards,
            "s05_render" => DramaWorkerTaskType::GenerateVideos,
            _ => DramaWorkerTaskType::GenerateOutline,
        };
        let result = dispatch_followup_task(
            dispatcher,
            &projection,
            &row,
            &row.project_id,
            task_type,
            stage,
            "resume_after_clarify",
            row.interaction_version as i64,
            payload,
        )
        .await;
        return match result {
            Ok((_job_id, _task)) => gateway_response(
                200,
                serde_json::json!({
                    "project_id": row.project_id,
                    "status": "running",
                    "accepted": true,
                    "interaction_version": row.interaction_version
                }),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response(),
        };
    }
    let body = serde_json::to_value(&req).unwrap_or_default();
    match facade.clarify(&project_id, body, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e).into_response(),
    }
}

pub async fn strategy_select(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(dispatcher): Extension<Option<DramaWorkerDispatcher>>,
    Extension(facade): Extension<DramaFacade>,
    Path(project_id): Path<String>,
    Json(req): Json<DramaStrategySelectRequest>,
) -> impl IntoResponse {
    if direct_worker_mode_enabled() {
        let row = match projection.get_projection(&project_id) {
            Ok(row) => row,
            Err(e) => {
                tracing::error!("Projection read failed: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e })),
                )
                    .into_response();
            }
        };
        let row = match check_projection_access(row, &user) {
            Ok(row) => row,
            Err(resp) => return resp,
        };
        if let Some(version) = req.interaction_version {
            if version != row.interaction_version as i64 {
                return stale_interaction_response();
            }
        }
        let dispatcher = match dispatcher.as_ref() {
            Some(dispatcher) => dispatcher,
            None => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({ "error": "dispatcher unavailable" })),
                )
                    .into_response();
            }
        };
        if let Err(e) = projection.mark_running(&row.project_id, Some("s03_script")) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response();
        }
        let payload = serde_json::json!({
            "project_id": row.project_id,
            "selected_option_id": req.selected_option_id,
            "source": "direct_worker_mode"
        });
        let result = dispatch_followup_task(
            dispatcher,
            &projection,
            &row,
            &row.project_id,
            DramaWorkerTaskType::GenerateEpisodeScripts,
            "s03_script",
            "generate_episode_scripts",
            row.interaction_version as i64,
            payload,
        )
        .await;
        return match result {
            Ok((_job_id, _task)) => gateway_response(
                200,
                serde_json::json!({
                    "project_id": row.project_id,
                    "status": "running",
                    "accepted": true,
                    "interaction_version": row.interaction_version
                }),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response(),
        };
    }
    let body = serde_json::to_value(&req).unwrap_or_default();
    match facade.strategy_select(&project_id, body, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e).into_response(),
    }
}

pub async fn approve(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(dispatcher): Extension<Option<DramaWorkerDispatcher>>,
    Extension(facade): Extension<DramaFacade>,
    Path(project_id): Path<String>,
    Json(req): Json<DramaApproveRequest>,
) -> impl IntoResponse {
    if direct_worker_mode_enabled() {
        let row = match projection.get_projection(&project_id) {
            Ok(row) => row,
            Err(e) => {
                tracing::error!("Projection read failed: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e })),
                )
                    .into_response();
            }
        };
        let row = match check_projection_access(row, &user) {
            Ok(row) => row,
            Err(resp) => return resp,
        };
        if let Some(version) = req.interaction_version {
            if version != row.interaction_version as i64 {
                return stale_interaction_response();
            }
        }
        let dispatcher = match dispatcher.as_ref() {
            Some(dispatcher) => dispatcher,
            None => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({ "error": "dispatcher unavailable" })),
                )
                    .into_response();
            }
        };

        let (task_type, stage_code, job_type, payload) = if req.approved {
            (
                DramaWorkerTaskType::PublishArtifacts,
                "s05_render",
                "publish_artifacts",
                serde_json::json!({
                    "project_id": row.project_id,
                    "decision": "approve",
                    "source": "direct_worker_mode"
                }),
            )
        } else {
            (
                DramaWorkerTaskType::GenerateEpisodeScripts,
                "s03_script",
                "generate_episode_scripts",
                serde_json::json!({
                    "project_id": row.project_id,
                    "decision": req.decision,
                    "revision_notes": req.revision_notes,
                    "source": "direct_worker_mode"
                }),
            )
        };

        if let Err(e) = projection.mark_running(&row.project_id, Some(stage_code)) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response();
        }

        let result = dispatch_followup_task(
            dispatcher,
            &projection,
            &row,
            &row.project_id,
            task_type,
            stage_code,
            job_type,
            row.interaction_version as i64,
            payload,
        )
        .await;
        return match result {
            Ok((_job_id, _task)) => gateway_response(
                200,
                serde_json::json!({
                    "project_id": row.project_id,
                    "status": "running",
                    "accepted": true,
                    "interaction_version": row.interaction_version
                }),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response(),
        };
    }
    let body = serde_json::to_value(&req).unwrap_or_default();
    match facade.approve(&project_id, body, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e).into_response(),
    }
}

pub async fn get_script(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(facade): Extension<DramaFacade>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match projection.get_projection(&project_id) {
        Ok(row) => {
            let row = match check_projection_access(row, &user) {
                Ok(row) => row,
                Err(resp) => return resp,
            };
            if let Some(ref payload) = row.interaction_payload {
                if payload.get("type").and_then(|v| v.as_str()) == Some("script_approval") {
                    return gateway_response(
                        200,
                        serde_json::json!({
                            "project_id": row.project_id,
                            "script": payload.get("script").cloned().unwrap_or(Value::Null)
                        }),
                    )
                    .into_response();
                }
                if let Some(script_data) = payload.get("script_data") {
                    if let Some(script) = script_data.get("script") {
                        return gateway_response(
                            200,
                            serde_json::json!({
                                "project_id": row.project_id,
                                "script": script
                            }),
                        )
                        .into_response();
                    }
                }
            }
        }
        Err(e) => tracing::error!("Projection read failed: {}", e),
    }
    match facade.get_script(&project_id, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e).into_response(),
    }
}

pub async fn get_shots(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(facade): Extension<DramaFacade>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match projection.get_projection(&project_id) {
        Ok(row) => {
            let row = match check_projection_access(row, &user) {
                Ok(row) => row,
                Err(resp) => return resp,
            };
            if let Some(ref payload) = row.interaction_payload {
                if let Some(shots_data) = payload.get("shots_data") {
                    let shots = shots_data
                        .get("shots")
                        .cloned()
                        .unwrap_or(Value::Array(vec![]));
                    let visual_style = shots_data
                        .get("visual_style")
                        .cloned()
                        .unwrap_or(serde_json::json!({}));
                    return gateway_response(
                        200,
                        serde_json::json!({
                            "project_id": row.project_id,
                            "shots": shots,
                            "visual_style": visual_style
                        }),
                    )
                    .into_response();
                }
            }
            let shots = row
                .interaction_payload
                .as_ref()
                .and_then(|payload| {
                    if payload.get("type").and_then(|v| v.as_str()) == Some("script_approval") {
                        payload
                            .get("script")
                            .and_then(|v| v.get("scenes"))
                            .and_then(|v| v.as_array())
                            .map(|scenes| {
                                scenes
                                    .iter()
                                    .map(|scene| {
                                        serde_json::json!({
                                            "shot_id": scene.get("scene_id").cloned().unwrap_or(Value::String(Uuid::new_v4().to_string())),
                                            "scene_id": scene.get("scene_id").cloned().unwrap_or(Value::Null),
                                            "sequence": scene.get("sequence").cloned().unwrap_or(Value::from(1)),
                                            "shot_type": "story",
                                            "duration_seconds": scene.get("duration_seconds").cloned().unwrap_or(Value::from(5)),
                                            "description": scene.get("action_summary").cloned().unwrap_or(Value::Null)
                                        })
                                    })
                                    .collect::<Vec<Value>>()
                            })
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            return gateway_response(
                200,
                serde_json::json!({
                    "project_id": row.project_id,
                    "shots": shots,
                    "visual_style": {}
                }),
            )
            .into_response();
        }
        Err(e) => tracing::error!("Projection read failed: {}", e),
    }

    match facade.get_shots(&project_id, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e).into_response(),
    }
}

pub async fn get_render(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(facade): Extension<DramaFacade>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match projection.get_projection(&project_id) {
        Ok(row) => {
            let row = match check_projection_access(row, &user) {
                Ok(row) => row,
                Err(resp) => return resp,
            };

            let render_tasks: Vec<Value> = projection
                .list_render_events(&row.project_id)
                .unwrap_or_default()
                .into_iter()
                .map(|evt| {
                    let segment_index = evt
                        .payload
                        .get("segment_index")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    let total_segments = evt
                        .payload
                        .get("total_segments")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    let segment_status = evt
                        .payload
                        .get("segment_status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    serde_json::json!({
                        "task_id": format!("render-seg{}", segment_index),
                        "shot_id": format!("shot-{}", segment_index),
                        "status": segment_status,
                        "segment_index": segment_index,
                        "total_segments": total_segments,
                    })
                })
                .collect();

            return gateway_response(
                200,
                serde_json::json!({
                    "project_id": row.project_id,
                    "render_tasks": render_tasks,
                    "progress_percent": row.progress_percent
                }),
            )
            .into_response();
        }
        Err(e) => tracing::error!("Projection read failed: {}", e),
    }
    match facade.get_render(&project_id, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e).into_response(),
    }
}

pub async fn get_artifacts(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(facade): Extension<DramaFacade>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match projection.get_projection(&project_id) {
        Ok(row) => {
            let row = match check_projection_access(row, &user) {
                Ok(row) => row,
                Err(resp) => return resp,
            };
            match projection.list_artifacts(&row.project_id) {
                Ok(rows) => {
                    let artifacts: Vec<Value> = rows
                        .into_iter()
                        .map(|item| {
                            let artifact_type = item
                                .metadata
                                .as_ref()
                                .and_then(|v| v.get("artifact_type"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("artifact");
                            serde_json::json!({
                                "artifact_id": item.id.to_string(),
                                "artifact_type": artifact_type,
                                "url": item.public_url,
                                "format": item.format,
                                "size_bytes": item.file_size,
                                "duration_seconds": item.duration_seconds
                            })
                        })
                        .collect();
                    let final_master = artifacts
                        .iter()
                        .find(|item| {
                            item.get("artifact_type")
                                .and_then(|v| v.as_str())
                                .map(|s| s.contains("final") || s.contains("master"))
                                .unwrap_or(false)
                        })
                        .cloned()
                        .or_else(|| artifacts.first().cloned());
                    let audio_tracks: Vec<Value> = artifacts
                        .iter()
                        .filter(|item| {
                            item.get("artifact_type")
                                .and_then(|v| v.as_str())
                                .map(|s| s.contains("audio") || s.contains("narration"))
                                .unwrap_or(false)
                        })
                        .cloned()
                        .collect();
                    let subtitle_bundles: Vec<Value> = artifacts
                        .iter()
                        .filter(|item| {
                            item.get("artifact_type")
                                .and_then(|v| v.as_str())
                                .map(|s| s.contains("subtitle") || s.contains("srt"))
                                .unwrap_or(false)
                        })
                        .cloned()
                        .collect();
                    return gateway_response(
                        200,
                        serde_json::json!({
                            "project_id": row.project_id,
                            "artifacts": artifacts,
                            "final_master": final_master,
                            "audio_tracks": audio_tracks,
                            "subtitle_bundles": subtitle_bundles,
                            "stage_packages": []
                        }),
                    )
                    .into_response();
                }
                Err(e) => tracing::error!("Projection artifacts read failed: {}", e),
            }
        }
        Err(e) => tracing::error!("Projection read failed: {}", e),
    }
    match facade.get_artifacts(&project_id, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e).into_response(),
    }
}

pub async fn get_cost(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(facade): Extension<DramaFacade>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match projection.get_projection(&project_id) {
        Ok(row) => {
            let row = match check_projection_access(row, &user) {
                Ok(row) => row,
                Err(resp) => return resp,
            };
            match projection.list_cost_ledger(&row.project_id) {
                Ok(rows) => {
                    let total_cents: i64 = rows.iter().map(|item| item.amount_cents).sum();
                    let costs: Vec<Value> = rows
                        .into_iter()
                        .map(|item| {
                            serde_json::json!({
                                "cost_type": item.cost_type,
                                "amount_cents": item.amount_cents,
                                "source": item.provider.unwrap_or_else(|| "worker".to_string()),
                                "stage_code": item.stage_code.unwrap_or_default()
                            })
                        })
                        .collect();
                    return gateway_response(
                        200,
                        serde_json::json!({
                            "project_id": row.project_id,
                            "costs": costs,
                            "total_cents": total_cents
                        }),
                    )
                    .into_response();
                }
                Err(e) => tracing::error!("Projection cost read failed: {}", e),
            }
        }
        Err(e) => tracing::error!("Projection read failed: {}", e),
    }
    match facade.get_cost(&project_id, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e).into_response(),
    }
}

pub async fn get_fallbacks(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(facade): Extension<DramaFacade>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match projection.get_projection(&project_id) {
        Ok(row) => {
            let row = match check_projection_access(row, &user) {
                Ok(row) => row,
                Err(resp) => return resp,
            };
            match projection.list_fallback_events(&row.project_id) {
                Ok(rows) => {
                    let fallback_events: Vec<Value> = rows
                        .into_iter()
                        .map(|item| {
                            serde_json::json!({
                                "stage_code": item.stage_code,
                                "reason_category": item.reason_category,
                                "reason_detail": item.reason_detail,
                                "from_provider": item.from_provider,
                                "to_provider": item.to_provider,
                                "created_at": item.occurred_at.and_utc().to_rfc3339(),
                                "payload": item.payload
                            })
                        })
                        .collect();
                    return gateway_response(
                        200,
                        serde_json::json!({
                            "project_id": row.project_id,
                            "fallback_events": fallback_events
                        }),
                    )
                    .into_response();
                }
                Err(e) => tracing::error!("Projection fallback read failed: {}", e),
            }
        }
        Err(e) => tracing::error!("Projection read failed: {}", e),
    }
    match facade.get_fallbacks(&project_id, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e).into_response(),
    }
}

pub async fn script_feedback(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(facade): Extension<DramaFacade>,
    Path(project_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Response<BoxBody> {
    let projection_row = match projection.get_projection(&project_id) {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Projection read failed: {}", e);
            return internal_error_response("failed to read drama project", "读取短剧项目失败");
        }
    };
    if let Err(resp) = check_projection_access(projection_row, &user) {
        return resp;
    }

    match facade.post_script_feedback(&project_id, req, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e),
    }
}

pub async fn retry_project(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(facade): Extension<DramaFacade>,
    Path(project_id): Path<String>,
) -> Response<BoxBody> {
    let projection_row = match projection.get_projection(&project_id) {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Projection read failed: {}", e);
            return internal_error_response("failed to read drama project", "读取短剧项目失败");
        }
    };
    if let Err(resp) = check_projection_access(projection_row, &user) {
        return resp;
    }

    let body = serde_json::json!({"project_id": project_id});
    match facade.retry(&project_id, body, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e),
    }
}

pub async fn clone_project(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(facade): Extension<DramaFacade>,
    Path(project_id): Path<String>,
) -> Response<BoxBody> {
    let projection_row = match projection.get_projection(&project_id) {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Projection read failed: {}", e);
            return internal_error_response("failed to read drama project", "读取短剧项目失败");
        }
    };
    if let Err(resp) = check_projection_access(projection_row, &user) {
        return resp;
    }

    let body = serde_json::json!({"source_project_id": project_id});
    match facade.clone_project(&project_id, body, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e),
    }
}

pub async fn scene_rerun(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(facade): Extension<DramaFacade>,
    Path((project_id, scene_id)): Path<(String, String)>,
) -> Response<BoxBody> {
    let projection_row = match projection.get_projection(&project_id) {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Projection read failed: {}", e);
            return internal_error_response("failed to read drama project", "读取短剧项目失败");
        }
    };
    if let Err(resp) = check_projection_access(projection_row, &user) {
        return resp;
    }

    let body = serde_json::json!({"scene_id": scene_id});
    match facade.scene_rerun(&project_id, body, &user).await {
        Ok((status, body)) => gateway_response(status, body),
        Err(e) => map_facade_err(e),
    }
}

const MAX_IMAGE_FILE_SIZE: usize = 10 * 1024 * 1024;

const ALLOWED_IMAGE_CONTENT_TYPES: &[&str] = &[
    "image/jpeg",
    "image/jpg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/bmp",
];

pub async fn upload_image(
    Extension(user): Extension<User>,
    mut multipart: Multipart,
) -> Result<ApiResult<UploadImageResponse>, ApiError> {
    use crate::service::oss_service::{OssConfig, OssService};

    if !OssConfig::is_configured() {
        return Err(ApiError::InternalServerError(
            "OSS service not configured".to_string(),
        ));
    }

    let mut image_data: Option<Vec<u8>> = None;
    let mut image_filename: Option<String> = None;
    let mut image_content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::BusinessError(BusinessError::FormParsingFailed))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "file" || field_name == "image" {
            image_filename = field.file_name().map(|s| s.to_string());
            image_content_type = field.content_type().map(|s| s.to_string());
            let data = field.bytes().await.map_err(|e| {
                tracing::error!("Failed to read drama image data: {:?}", e);
                ApiError::BusinessError(BusinessError::InvalidFormField("file".to_string()))
            })?;
            image_data = Some(data.to_vec());
        }
    }

    let data = image_data.ok_or_else(|| {
        ApiError::BusinessError(BusinessError::MissingRequiredParameter("file".to_string()))
    })?;
    let filename = image_filename.unwrap_or_else(|| "image.jpg".to_string());
    let content_type = image_content_type.unwrap_or_else(|| "image/jpeg".to_string());

    if !ALLOWED_IMAGE_CONTENT_TYPES.contains(&content_type.as_str()) {
        return Err(ApiError::BusinessError(BusinessError::InvalidFileType(
            content_type,
        )));
    }
    if data.len() > MAX_IMAGE_FILE_SIZE {
        return Err(ApiError::BusinessError(BusinessError::FileTooLarge(
            MAX_IMAGE_FILE_SIZE,
        )));
    }

    tracing::info!(
        "Drama upload-image: filename={}, size={} bytes, user_id={}",
        filename,
        data.len(),
        user.id
    );

    let oss_service = OssService::from_env()?;
    let result = oss_service
        .upload_image(data, user.id, filename.clone(), content_type.clone())
        .await?;

    Ok(ApiResult::ok(UploadImageResponse {
        image_url: result.image_url,
        filename: result.filename,
        size: result.size,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;

    fn sample_projection_row() -> ProjectionRow {
        ProjectionRow {
            id: 1,
            project_id: "proj_001".to_string(),
            user_id: 7,
            title: "The Last Firewall".to_string(),
            description: "Cyberpunk control-plane test".to_string(),
            status: "needs_approval".to_string(),
            content_type: Some("short_video".to_string()),
            platform: Some("tiktok".to_string()),
            current_stage: Some("s01_strategy".to_string()),
            pending_stage: Some("strategy".to_string()),
            progress_percent: 15.0,
            run_id: Some("run_001".to_string()),
            interaction_version: 2,
            last_event_sequence: 9,
            error_message: None,
            cost_reserve_cents: 5000,
            cost_consumed_cents: 1200,
            interaction_payload: Some(json!({
                "type": "strategy_selection",
                "packages": []
            })),
            completed_at: None,
            created_at: NaiveDate::from_ymd_opt(2026, 3, 24)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2026, 3, 24)
                .unwrap()
                .and_hms_opt(12, 5, 0)
                .unwrap(),
        }
    }

    #[test]
    fn projection_summary_json_includes_expected_fields() {
        let row = sample_projection_row();
        let value = projection_summary_json(&row);
        assert_eq!(value.get("project_id"), Some(&json!("proj_001")));
        assert_eq!(value.get("status"), Some(&json!("needs_approval")));
        assert_eq!(value.pointer("/cost/estimated_cents"), Some(&json!(5000)));
        assert_eq!(value.pointer("/cost/consumed_cents"), Some(&json!(1200)));
    }

    #[test]
    fn projection_detail_json_includes_interaction_payload() {
        let row = sample_projection_row();
        let value = projection_detail_json(&row);
        assert_eq!(value.get("project_id"), Some(&json!("proj_001")));
        assert_eq!(value.get("pending_stage"), Some(&json!("strategy")));
        assert_eq!(
            value.pointer("/interaction/type"),
            Some(&json!("strategy_selection"))
        );
        assert_eq!(
            value.pointer("/cost_summary/reserve_cents"),
            Some(&json!(5000))
        );
    }

    #[test]
    fn project_meta_response_includes_updated_at() {
        let updated_at = NaiveDate::from_ymd_opt(2026, 3, 27)
            .unwrap()
            .and_hms_opt(9, 30, 0)
            .unwrap();
        let response = project_meta_response_from_row(DramaProjectMetaRow {
            project_id: "proj_meta_001".to_string(),
            user_id: 7,
            characters: json!([]),
            style_references: json!([]),
            text_materials: json!([]),
            visual_settings: json!({"palette": "neon"}),
            created_at: updated_at,
            updated_at,
        });

        assert_eq!(response.project_id, "proj_meta_001");
        assert_eq!(response.updated_at, "2026-03-27T09:30:00+00:00");
    }
}
