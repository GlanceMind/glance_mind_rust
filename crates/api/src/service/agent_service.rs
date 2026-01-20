use crate::dto::agent_dto::{
    CommentWithVideoDto, DeviceCommentsQuery, UnifiedCommentDto, UpdateCommentStatusDto,
    UpdateStatusResponse,
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

    // New method for device-based query
    pub async fn get_comments_by_device(
        &self,
        query: DeviceCommentsQuery,
    ) -> Result<PageResponse<CommentWithVideoDto>, ApiError> {
        self.agent_repo
            .get_comments_by_device(&query.device_id, query.status, query.page, query.per_page)
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
