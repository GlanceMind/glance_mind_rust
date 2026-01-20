use crate::config::database::Database;
use crate::dto::crawler_dto::{CrawlerResultDto, CrawlerTaskDto};
use crate::error::api_error::ApiError;
use crate::repository::crawler_repository::CrawlerRepository;
use glance_mind_db::entity::crawler::{CrawlerResult, CrawlerTask};
use std::sync::Arc;

#[derive(Clone)]
pub struct CrawlerService {
    repo: CrawlerRepository,
}

impl CrawlerService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            repo: CrawlerRepository::new(db.pool.clone()),
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
    ) -> Result<crate::dto::common::PageResponse<CrawlerResultDto>, ApiError> {
        let (results, total) = self
            .repo
            .find_results_by_task_id(task_id, req.page, req.page_size)
            .await
            .map_err(|e| {
                ApiError::InternalServerError(format!("Failed to fetch crawler results: {}", e))
            })?;

        let dtos = results.into_iter().map(Self::result_to_dto).collect();
        Ok(crate::dto::common::PageResponse::new(
            dtos,
            total,
            req.page,
            req.page_size,
        ))
    }

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

    fn task_to_dto(task: CrawlerTask) -> CrawlerTaskDto {
        CrawlerTaskDto {
            id: task.id,
            campaign_id: task.campaign_id,
            keywords: task.keywords.map(|k| k.into_iter().flatten().collect()),
            max_count: task.max_count,
            process_count: task.process_count,
            status: task.status,
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
