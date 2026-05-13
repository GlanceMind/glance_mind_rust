use crate::config::database::Database;
use crate::dto::crawler_dto::{CrawlerResultDto, CrawlerTaskDto, UnifiedContentDto};
use crate::error::api_error::ApiError;
use crate::repository::campaign_repository::CampaignRepository;
use crate::repository::crawler_repository::CrawlerRepository;
use diesel::result::Error as DieselError;
use glance_mind_db::entity::crawler::{CrawlerResult, CrawlerTask};
use std::sync::Arc;

#[derive(Clone)]
pub struct CrawlerService {
    repo: CrawlerRepository,
    campaign_repo: CampaignRepository,
}

impl CrawlerService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            repo: CrawlerRepository::new(db.pool.clone()),
            campaign_repo: CampaignRepository::new(db.pool.clone()),
        }
    }

    pub async fn list_campaign_tasks(
        &self,
        campaign_id: i32,
        req: crate::dto::common::PageRequest,
    ) -> Result<crate::dto::common::PageResponse<CrawlerTaskDto>, ApiError> {
        let (tasks, total) = self
            .repo
            .find_tasks_by_campaign_id(campaign_id, req.page, req.page_size)
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to fetch crawler tasks: {}", e))
            })?;

        let dtos = tasks.into_iter().map(Self::task_to_dto).collect();
        Ok(crate::dto::common::PageResponse::new(
            dtos,
            total,
            req.page,
            req.page_size,
        ))
    }

    pub async fn list_task_results(
        &self,
        task_id: i32,
        req: crate::dto::common::PageRequest,
    ) -> Result<crate::dto::common::PageResponse<UnifiedContentDto>, ApiError> {
        let (contents, total) = self
            .repo
            .find_unified_contents_by_task(task_id, req.page, req.page_size)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::NotFound(format!("Crawler task {} not found", task_id))
                }
                _ => {
                    ApiError::InternalServerError(format!("Failed to fetch crawler results: {}", e))
                }
            })?;

        Ok(crate::dto::common::PageResponse::new(
            contents,
            total,
            req.page,
            req.page_size,
        ))
    }

    /// Legacy method - returns CrawlerResultDto for backward compatibility (TikTok only)
    #[allow(dead_code)]
    pub async fn list_campaign_results(
        &self,
        campaign_id: i32,
        req: crate::dto::common::PageRequest,
    ) -> Result<crate::dto::common::PageResponse<CrawlerResultDto>, ApiError> {
        let (results, total) = self
            .repo
            .find_results_by_campaign_id(campaign_id, req.page, req.page_size)
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!(
                    "Failed to fetch campaign crawler results: {}",
                    e
                ))
            })?;

        let dtos = results.into_iter().map(Self::result_to_dto).collect();
        Ok(crate::dto::common::PageResponse::new(
            dtos,
            total,
            req.page,
            req.page_size,
        ))
    }

    /// Unified method - automatically detects platform and returns UnifiedContentDto
    /// This is the main method used by the API endpoints
    pub async fn list_campaign_results_unified(
        &self,
        campaign_id: i32,
        req: crate::dto::common::PageRequest,
    ) -> Result<crate::dto::common::PageResponse<UnifiedContentDto>, ApiError> {
        // Get campaign to determine platform_id
        let campaign = self
            .campaign_repo
            .find_by_id(campaign_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::NotFound(format!("Campaign {} not found", campaign_id))
                }
                _ => ApiError::InternalServerError(format!("Failed to fetch campaign: {}", e)),
            })?;

        tracing::info!(
            "list_campaign_results_unified: campaign_id={}, platform_id={}",
            campaign_id,
            campaign.platform_id
        );

        let (contents, total) = self
            .repo
            .find_unified_contents_by_campaign(
                campaign_id,
                campaign.platform_id,
                req.page,
                req.page_size,
            )
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::InternalServerError(format!(
                    "Unsupported campaign platform_id: {}",
                    campaign.platform_id
                )),
                _ => ApiError::InternalServerError(format!(
                    "Failed to fetch campaign contents: {}",
                    e
                )),
            })?;

        tracing::info!(
            "list_campaign_results_unified: found {} contents, total={}",
            contents.len(),
            total
        );

        Ok(crate::dto::common::PageResponse::new(
            contents,
            total,
            req.page,
            req.page_size,
        ))
    }

    /// Unified method with explicit platform_id - returns UnifiedContentDto based on platform
    #[allow(dead_code)]
    pub async fn list_campaign_contents_unified(
        &self,
        campaign_id: i32,
        platform_id: i32,
        req: crate::dto::common::PageRequest,
    ) -> Result<crate::dto::common::PageResponse<UnifiedContentDto>, ApiError> {
        let campaign = self
            .campaign_repo
            .find_by_id(campaign_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::NotFound(format!("Campaign {} not found", campaign_id))
                }
                _ => ApiError::InternalServerError(format!("Failed to fetch campaign: {}", e)),
            })?;

        if campaign.platform_id != platform_id {
            return Err(ApiError::BadRequest(format!(
                "platform_id {} does not match campaign {} platform_id {}",
                platform_id, campaign_id, campaign.platform_id
            )));
        }

        let (contents, total) = self
            .repo
            .find_unified_contents_by_campaign(
                campaign_id,
                campaign.platform_id,
                req.page,
                req.page_size,
            )
            .await
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::BadRequest(format!("Invalid platform_id: {}", campaign.platform_id))
                }
                _ => ApiError::InternalServerError(format!(
                    "Failed to fetch campaign contents: {}",
                    e
                )),
            })?;

        Ok(crate::dto::common::PageResponse::new(
            contents,
            total,
            req.page,
            req.page_size,
        ))
    }

    pub async fn get_all_campaign_contents_unified(
        &self,
        campaign_id: i32,
        platform_id: i32,
    ) -> Result<Vec<UnifiedContentDto>, ApiError> {
        let campaign = self
            .campaign_repo
            .find_by_id(campaign_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::NotFound(format!("Campaign {} not found", campaign_id))
                }
                _ => ApiError::InternalServerError(format!("Failed to fetch campaign: {}", e)),
            })?;

        if campaign.platform_id != platform_id {
            return Err(ApiError::BadRequest(format!(
                "platform_id {} does not match campaign {} platform_id {}",
                platform_id, campaign_id, campaign.platform_id
            )));
        }

        self.repo
            .find_unified_contents_by_campaign(campaign_id, campaign.platform_id, 1, i64::MAX)
            .await
            .map(|(contents, _)| contents)
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::BadRequest(format!("Invalid platform_id: {}", campaign.platform_id))
                }
                _ => ApiError::InternalServerError(format!(
                    "Failed to fetch campaign contents: {}",
                    e
                )),
            })
    }

    fn task_to_dto(task: CrawlerTask) -> CrawlerTaskDto {
        CrawlerTaskDto {
            id: task.id,
            campaign_id: task.campaign_id,
            keywords: task.keywords.map(|k| k.into_iter().flatten().collect()),
            max_count: task.max_count,
            process_count: task.process_count,
            status: task.status,
            terminal_reason: task.terminal_reason,
            created_at: task.created_at,
            updated_at: task.updated_at,
        }
    }

    fn result_to_dto(result: CrawlerResult) -> CrawlerResultDto {
        CrawlerResultDto {
            id: result.id,
            task_id: result.task_id,
            video_id: result.video_id,
            video_title: result.video_title,
            comment_count: result.comment_count,
            view_count: result.view_count,
            like_count: result.like_count,
            // share_count and play_count are not in the database schema
            // They are DTO-only fields for API compatibility
            share_count: None,
            play_count: result.view_count, // Use view_count as play_count for compatibility
            author_name: result.author_name,
            processed: result.processed,
            replied: result.replied,
            created_at: result.created_at,
        }
    }
}
