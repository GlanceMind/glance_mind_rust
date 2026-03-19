use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;

use crate::dto::video_case_dto::{
    VideoCaseDetail, VideoCaseListItem, VideoCaseListQuery, VideoCaseListResponse,
};
use crate::error::api_error::ApiError;
use crate::error::db_error::DbError;
use crate::repository::video_case_repository::VideoCaseRepository;

#[derive(Clone)]
pub struct VideoCaseService {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl VideoCaseService {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    /// Get video case list with pagination
    pub async fn list(&self, query: VideoCaseListQuery) -> Result<VideoCaseListResponse, ApiError> {
        let pool = self.pool.clone();

        let result =
            tokio::task::spawn_blocking(move || -> Result<VideoCaseListResponse, ApiError> {
                let mut conn = pool.get().map_err(|e| {
                    tracing::error!("Failed to get DB connection: {:?}", e);
                    ApiError::DbError(DbError::SomethingWentWrong(format!(
                        "Connection failed: {}",
                        e
                    )))
                })?;

                let page = query.page.unwrap_or(1).max(1);
                let page_size = query.page_size.unwrap_or(20).min(100);

                let (cases, total) = VideoCaseRepository::list(
                    &mut conn,
                    page,
                    page_size,
                    query.status,
                    query.video_status,
                    query.category_id,
                    query.user_id,
                )
                .map_err(|e| {
                    ApiError::DbError(DbError::SomethingWentWrong(format!("Query failed: {}", e)))
                })?;

                let items: Vec<VideoCaseListItem> = cases.into_iter().map(|c| c.into()).collect();

                Ok(VideoCaseListResponse {
                    items,
                    total,
                    page,
                    page_size,
                })
            })
            .await
            .map_err(|e| {
                tracing::error!("Task join error: {:?}", e);
                ApiError::DbError(DbError::SomethingWentWrong(format!(
                    "Task join error: {}",
                    e
                )))
            })??;

        Ok(result)
    }

    /// Get video case detail by video_id
    pub async fn get_by_video_id(&self, video_id: i64) -> Result<VideoCaseDetail, ApiError> {
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || -> Result<VideoCaseDetail, ApiError> {
            let mut conn = pool.get().map_err(|e| {
                tracing::error!("Failed to get DB connection: {:?}", e);
                ApiError::DbError(DbError::SomethingWentWrong(format!(
                    "Connection failed: {}",
                    e
                )))
            })?;

            let case = VideoCaseRepository::get_by_video_id(&mut conn, video_id).map_err(|e| {
                ApiError::DbError(DbError::SomethingWentWrong(format!(
                    "Video case {} not found: {}",
                    video_id, e
                )))
            })?;

            Ok(case.into())
        })
        .await
        .map_err(|e| {
            tracing::error!("Task join error: {:?}", e);
            ApiError::DbError(DbError::SomethingWentWrong(format!(
                "Task join error: {}",
                e
            )))
        })??;

        Ok(result)
    }

    /// Get video case detail by task_no
    pub async fn get_by_task_no(&self, task_no: &str) -> Result<VideoCaseDetail, ApiError> {
        let pool = self.pool.clone();
        let task_no = task_no.to_string();

        let result = tokio::task::spawn_blocking(move || -> Result<VideoCaseDetail, ApiError> {
            let mut conn = pool.get().map_err(|e| {
                tracing::error!("Failed to get DB connection: {:?}", e);
                ApiError::DbError(DbError::SomethingWentWrong(format!(
                    "Connection failed: {}",
                    e
                )))
            })?;

            let case = VideoCaseRepository::get_by_task_no(&mut conn, &task_no).map_err(|e| {
                ApiError::DbError(DbError::SomethingWentWrong(format!(
                    "Video case {} not found: {}",
                    task_no, e
                )))
            })?;

            Ok(case.into())
        })
        .await
        .map_err(|e| {
            tracing::error!("Task join error: {:?}", e);
            ApiError::DbError(DbError::SomethingWentWrong(format!(
                "Task join error: {}",
                e
            )))
        })??;

        Ok(result)
    }
}
