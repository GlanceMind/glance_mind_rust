use bigdecimal::BigDecimal;
use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::PgConnection as DieselPgConnection;

use glance_mind_db::entity::video::{
    NewVideoGenerationTask, UpdateVideoGenerationTask, VideoGenerationTask,
};
use glance_mind_db::schema::gm_video_generation_tasks;

pub type PgConnection = PooledConnection<ConnectionManager<DieselPgConnection>>;

pub struct VideoRepository;

impl VideoRepository {
    /// Create new video generation task
    pub fn create_task(
        conn: &mut PgConnection,
        new_task: NewVideoGenerationTask,
    ) -> Result<VideoGenerationTask, diesel::result::Error> {
        diesel::insert_into(gm_video_generation_tasks::table)
            .values(&new_task)
            .get_result(conn)
            .map_err(|e| {
                tracing::error!("Failed to create video task: {:?}", e);
                e
            })
    }

    /// Query task by task_id
    pub fn get_by_task_id(
        conn: &mut PgConnection,
        task_id: &str,
    ) -> Result<VideoGenerationTask, diesel::result::Error> {
        gm_video_generation_tasks::table
            .filter(gm_video_generation_tasks::task_id.eq(task_id))
            .first(conn)
            .map_err(|e| {
                tracing::error!("Failed to query video task task_id={}: {:?}", task_id, e);
                e
            })
    }

    /// Query task by ID
    pub fn get_by_id(
        conn: &mut PgConnection,
        id: i32,
    ) -> Result<VideoGenerationTask, diesel::result::Error> {
        gm_video_generation_tasks::table
            .find(id)
            .first(conn)
            .map_err(|e| {
                tracing::error!("Failed to query video task id={}: {:?}", id, e);
                e
            })
    }

    /// Query all tasks for a user
    pub fn get_by_user_id(
        conn: &mut PgConnection,
        user_id: i32,
        status: Option<String>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<VideoGenerationTask>, i64), diesel::result::Error> {
        // Build filter for count query
        let mut count_query = gm_video_generation_tasks::table
            .filter(gm_video_generation_tasks::user_id.eq(user_id))
            .into_boxed();

        if let Some(ref status_filter) = status {
            count_query = count_query.filter(gm_video_generation_tasks::status.eq(status_filter));
        }

        let total = count_query.count().get_result::<i64>(conn)?;

        // Build query for tasks
        let mut tasks_query = gm_video_generation_tasks::table
            .filter(gm_video_generation_tasks::user_id.eq(user_id))
            .into_boxed();

        if let Some(status_filter) = status {
            tasks_query = tasks_query.filter(gm_video_generation_tasks::status.eq(status_filter));
        }

        let tasks = tasks_query
            .order(gm_video_generation_tasks::created_at.desc())
            .limit(page_size as i64)
            .offset(((page - 1) * page_size) as i64)
            .load::<VideoGenerationTask>(conn)
            .map_err(|e| {
                tracing::error!(
                    "Failed to query user video task list user_id={}: {:?}",
                    user_id,
                    e
                );
                e
            })?;

        Ok((tasks, total))
    }

    /// Query all incomplete tasks (for scheduler polling)
    pub fn get_active_tasks(
        conn: &mut PgConnection,
        limit: i64,
    ) -> Result<Vec<VideoGenerationTask>, diesel::result::Error> {
        let one_second_ago = Utc::now() - Duration::seconds(1);

        gm_video_generation_tasks::table
            .filter(gm_video_generation_tasks::status.eq_any(vec![
                "pending",
                "queued",
                "processing",
            ]))
            .filter(
                gm_video_generation_tasks::updated_at
                    .lt(one_second_ago)
                    .or(gm_video_generation_tasks::updated_at.is_null()),
            )
            .order(gm_video_generation_tasks::updated_at.asc())
            .limit(limit)
            .load::<VideoGenerationTask>(conn)
            .map_err(|e| {
                tracing::error!("Failed to query active video tasks: {:?}", e);
                e
            })
    }

    /// Query timed out tasks (not completed 24 hours after creation)
    pub fn get_timeout_tasks(
        conn: &mut PgConnection,
    ) -> Result<Vec<VideoGenerationTask>, diesel::result::Error> {
        let timeout_time = Utc::now() - Duration::hours(24);

        gm_video_generation_tasks::table
            .filter(gm_video_generation_tasks::status.eq_any(vec![
                "pending",
                "queued",
                "processing",
            ]))
            .filter(gm_video_generation_tasks::created_at.lt(timeout_time))
            .load::<VideoGenerationTask>(conn)
            .map_err(|e| {
                tracing::error!("Failed to query timed out video tasks: {:?}", e);
                e
            })
    }

    /// Update task
    pub fn update_task(
        conn: &mut PgConnection,
        id: i32,
        update: UpdateVideoGenerationTask,
    ) -> Result<VideoGenerationTask, diesel::result::Error> {
        diesel::update(gm_video_generation_tasks::table.find(id))
            .set(&update)
            .get_result(conn)
            .map_err(|e| {
                tracing::error!("Failed to update video task id={}: {:?}", id, e);
                e
            })
    }

    /// Update task status
    pub fn update_status(
        conn: &mut PgConnection,
        id: i32,
        status: String,
        progress_pct: Option<BigDecimal>,
    ) -> Result<VideoGenerationTask, diesel::result::Error> {
        let update = UpdateVideoGenerationTask {
            status: Some(status),
            progress_pct,
            updated_at: Some(Utc::now()),
            ..Default::default()
        };

        Self::update_task(conn, id, update)
    }

    /// Mark task as succeeded
    #[allow(clippy::too_many_arguments)]
    pub fn mark_as_succeeded(
        conn: &mut PgConnection,
        record_id: i32,
        generation_id_val: Option<String>,
        video_url_val: Option<String>,
        thumbnail_url_val: Option<String>,
        video_width_val: Option<i32>,
        video_height_val: Option<i32>,
        provider_post_id_val: Option<String>,
        provider_response_val: Option<serde_json::Value>,
    ) -> Result<VideoGenerationTask, diesel::result::Error> {
        use glance_mind_db::schema::gm_video_generation_tasks::dsl::*;

        diesel::update(gm_video_generation_tasks.filter(id.eq(record_id)))
            .set((
                status.eq("succeeded"),
                progress_pct.eq(Some(BigDecimal::from(1))), // 100% = 1.00
                generation_id.eq(generation_id_val),
                video_url.eq(video_url_val),
                thumbnail_url.eq(thumbnail_url_val),
                video_width.eq(video_width_val),
                video_height.eq(video_height_val),
                provider_post_id.eq(provider_post_id_val),
                provider_response.eq(provider_response_val),
                updated_at.eq(Utc::now()),
                completed_at.eq(Some(Utc::now())),
            ))
            .get_result(conn)
    }

    /// Mark task as failed
    pub fn mark_as_failed(
        conn: &mut PgConnection,
        id: i32,
        error_message: String,
    ) -> Result<VideoGenerationTask, diesel::result::Error> {
        let update = UpdateVideoGenerationTask {
            status: Some("failed".to_string()),
            error_message: Some(error_message),
            updated_at: Some(Utc::now()),
            completed_at: Some(Utc::now()),
            ..Default::default()
        };

        Self::update_task(conn, id, update)
    }

    /// Increment retry count
    pub fn increment_retry(
        conn: &mut PgConnection,
        task_id_param: i32,
    ) -> Result<VideoGenerationTask, diesel::result::Error> {
        use glance_mind_db::schema::gm_video_generation_tasks::dsl::*;

        let task = Self::get_by_id(conn, task_id_param)?;
        diesel::update(gm_video_generation_tasks.filter(id.eq(task_id_param)))
            .set((
                retry_count.eq(task.retry_count + 1),
                updated_at.eq(Utc::now()),
            ))
            .get_result(conn)
    }
}
