use crate::dto::agent_dto::{DeviceCommentsQuery, UpdateCommentStatusDto};
use crate::error::ServiceError;
use crate::service::agent_service::AgentService;
use actix_web::{web, HttpResponse, Scope};
use validator::Validate;

pub fn public_routes(agent_service: web::Data<AgentService>) -> Scope {
    web::scope("/public")
        .app_data(agent_service)
        .route("/comments/by-device", web::get().to(get_comments_by_device))
        .route(
            "/comments/update-status",
            web::post().to(update_comment_status),
        )
}

async fn get_comments_by_device(
    query: web::Query<DeviceCommentsQuery>,
    service: web::Data<AgentService>,
) -> Result<HttpResponse, ServiceError> {
    let result = service.get_comments_by_device(query.into_inner())?;
    Ok(HttpResponse::Ok().json(result))
}

async fn update_comment_status(
    dto: web::Json<UpdateCommentStatusDto>,
    service: web::Data<AgentService>,
) -> Result<HttpResponse, ServiceError> {
    dto.validate()
        .map_err(|e| ServiceError::ValidationError(e.to_string()))?;
    let result = service.update_comment_status(dto.into_inner())?;
    Ok(HttpResponse::Ok().json(result))
}
