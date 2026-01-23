use crate::dto::agent_dto::{
    CommentWithVideoDto, DeviceCommentsQuery, DeviceCommentsResponse, UnifiedCommentDto,
    UnifiedCommentWithConfigDto, UpdateCommentStatusDto, UpdateStatusResponse,
};
use crate::dto::common::{PageRequest, PageResponse};
use crate::error::api_error::ApiError;
use crate::repository::agent_repository::AgentRepository;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;

#[derive(Clone)]
pub struct AgentService {
    agent_repo: AgentRepository,
}

impl AgentService {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            agent_repo: AgentRepository::new(pool),
        }
    }

    /// Legacy method - only queries TikTok comments
    pub async fn get_video_comments(
        &self,
        video_id: i32,
        req: PageRequest,
    ) -> Result<PageResponse<crate::dto::agent_dto::AgentCommentDto>, ApiError> {
        self.agent_repo
            .get_video_comments(video_id, req.page, req.page_size)
            .map_err(|e| ApiError::InternalServerError(e.to_string()))
    }

    /// Unified method - queries comments for any platform
    pub async fn get_unified_comments(
        &self,
        content_db_id: i32,
        platform_id: i32,
        req: PageRequest,
    ) -> Result<PageResponse<UnifiedCommentDto>, ApiError> {
        self.agent_repo
            .get_unified_comments(content_db_id, platform_id, req.page, req.page_size)
            .map_err(|e| ApiError::InternalServerError(e.to_string()))
    }

    // New method for device-based query with platform support
    // Returns protocol-compliant structure: campaign config + comments array
    pub async fn get_comments_by_device(
        &self,
        query: DeviceCommentsQuery,
    ) -> Result<DeviceCommentsResponse, ApiError> {
        // Convert i32 to i64 for repository layer (database operations use i64)
        let page_i64 = i64::from(query.page);
        let per_page_i64 = i64::from(query.per_page);

        let page_response = self
            .agent_repo
            .get_comments_by_device_unified(
                &query.device_id,
                &query.platform,
                query.status,
                page_i64,
                per_page_i64,
            )
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        // Convert to protocol-compliant structure (uses i32 for pagination)
        Ok(UnifiedCommentWithConfigDto::to_protocol_response(
            page_response.list,
            page_response.total,
            query.page,
            query.per_page,
        ))
    }

    // Legacy method for backward compatibility (TikTok only)
    #[allow(dead_code)]
    pub async fn get_tiktok_comments_by_device(
        &self,
        query: DeviceCommentsQuery,
    ) -> Result<PageResponse<CommentWithVideoDto>, ApiError> {
        // Convert i32 to i64 for repository layer
        let page_i64 = i64::from(query.page);
        let per_page_i64 = i64::from(query.per_page);

        self.agent_repo
            .get_comments_by_device(&query.device_id, query.status, page_i64, per_page_i64)
            .map_err(|e| ApiError::InternalServerError(e.to_string()))
    }

    // New method for status update
    pub async fn update_comment_status(
        &self,
        dto: UpdateCommentStatusDto,
    ) -> Result<UpdateStatusResponse, ApiError> {
        let affected = self
            .agent_repo
            .update_comment_status(&dto.comment_id, dto.status)
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        if affected == 0 {
            return Err(ApiError::NotFound(format!(
                "Comment {} not found",
                dto.comment_id
            )));
        }

        Ok(UpdateStatusResponse {
            success: true,
            message: "Status updated successfully".to_string(),
        })
    }

    /// Get all videos for a campaign (for export)
    pub async fn get_campaign_videos(
        &self,
        campaign_id: i32,
    ) -> Result<Vec<glance_mind_db::entity::agent::AgentVideo>, ApiError> {
        self.agent_repo
            .get_campaign_videos(campaign_id)
            .map_err(|e| ApiError::InternalServerError(e.to_string()))
    }

    /// Get all comments for a campaign (for export)
    pub async fn get_campaign_comments(
        &self,
        campaign_id: i32,
    ) -> Result<Vec<glance_mind_db::entity::agent::AgentComment>, ApiError> {
        self.agent_repo
            .get_campaign_comments(campaign_id)
            .map_err(|e| ApiError::InternalServerError(e.to_string()))
    }
}
