use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::PgConnection as DieselPgConnection;
use diesel::SelectableHelper;
use std::convert::TryFrom;

use glance_mind_db::entity::video_case::{NewVideoCase, UpdateVideoCase, VideoCase};
use glance_mind_db::schema::gm_data_video_cases;

pub type PgConnection = PooledConnection<ConnectionManager<DieselPgConnection>>;

pub struct VideoCaseRepository;

impl VideoCaseRepository {
    /// Get video case by task_no (primary key)
    pub fn get_by_task_no(
        conn: &mut PgConnection,
        task_no_param: &str,
    ) -> Result<VideoCase, diesel::result::Error> {
        gm_data_video_cases::table
            .filter(gm_data_video_cases::task_no.eq(task_no_param))
            .select(VideoCase::as_select())
            .first(conn)
            .map_err(|e| {
                tracing::error!("Failed to get video case task_no={}: {:?}", task_no_param, e);
                e
            })
    }

    /// Get video case by row id.
    pub fn get_by_video_id(
        conn: &mut PgConnection,
        video_id_param: i64,
    ) -> Result<VideoCase, diesel::result::Error> {
        let case_id = i32::try_from(video_id_param).map_err(|_| diesel::result::Error::NotFound)?;

        gm_data_video_cases::table
            .filter(gm_data_video_cases::id.eq(case_id))
            .select(VideoCase::as_select())
            .first(conn)
            .map_err(|e| {
                tracing::error!("Failed to get video case id={}: {:?}", video_id_param, e);
                e
            })
    }

    /// List video cases with pagination and optional filters
    pub fn list(
        conn: &mut PgConnection,
        page: i32,
        page_size: i32,
        detail_status: Option<i32>,
        video_status: Option<String>,
        category_id: Option<String>,
        user_id: Option<String>,
    ) -> Result<(Vec<VideoCase>, i64), diesel::result::Error> {
        // Build count query
        let mut count_query = gm_data_video_cases::table.into_boxed();

        if let Some(s) = detail_status {
            count_query = count_query.filter(gm_data_video_cases::status.eq(s));
        }
        if let Some(ref vs) = video_status {
            count_query = count_query.filter(gm_data_video_cases::video_status.eq(vs));
        }
        if let Some(ref cat) = category_id {
            count_query = count_query.filter(gm_data_video_cases::tt_category_id.eq(cat));
        }
        if let Some(ref uid) = user_id {
            count_query = count_query.filter(gm_data_video_cases::user_id.eq(uid));
        }

        let total = count_query.count().get_result::<i64>(conn)?;

        // Build data query
        let mut data_query = gm_data_video_cases::table.into_boxed();

        if let Some(s) = detail_status {
            data_query = data_query.filter(gm_data_video_cases::status.eq(s));
        }
        if let Some(vs) = video_status {
            data_query = data_query.filter(gm_data_video_cases::video_status.eq(vs));
        }
        if let Some(cat) = category_id {
            data_query = data_query.filter(gm_data_video_cases::tt_category_id.eq(cat));
        }
        if let Some(uid) = user_id {
            data_query = data_query.filter(gm_data_video_cases::user_id.eq(uid));
        }

        let cases = data_query
            .order(gm_data_video_cases::created_at.desc())
            .limit(page_size as i64)
            .offset(((page - 1) * page_size) as i64)
            .select(VideoCase::as_select())
            .load(conn)
            .map_err(|e| {
                tracing::error!("Failed to list video cases: {:?}", e);
                e
            })?;

        Ok((cases, total))
    }

    /// Create a new video case
    pub fn create(
        conn: &mut PgConnection,
        new_case: NewVideoCase,
    ) -> Result<VideoCase, diesel::result::Error> {
        diesel::insert_into(gm_data_video_cases::table)
            .values(&new_case)
            .returning(VideoCase::as_select())
            .get_result(conn)
            .map_err(|e| {
                tracing::error!("Failed to create video case: {:?}", e);
                e
            })
    }

    /// Update a video case by task_no
    pub fn update(
        conn: &mut PgConnection,
        task_no_param: &str,
        update: UpdateVideoCase,
    ) -> Result<VideoCase, diesel::result::Error> {
        diesel::update(gm_data_video_cases::table.filter(gm_data_video_cases::task_no.eq(task_no_param)))
            .set(&update)
            .returning(VideoCase::as_select())
            .get_result(conn)
            .map_err(|e| {
                tracing::error!("Failed to update video case task_no={}: {:?}", task_no_param, e);
                e
            })
    }

    /// Delete a video case by task_no
    pub fn delete(conn: &mut PgConnection, task_no_param: &str) -> Result<usize, diesel::result::Error> {
        diesel::delete(gm_data_video_cases::table.filter(gm_data_video_cases::task_no.eq(task_no_param)))
            .execute(conn)
            .map_err(|e| {
                tracing::error!("Failed to delete video case task_no={}: {:?}", task_no_param, e);
                e
            })
    }
}
