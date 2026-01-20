use glance_mind_db::entity::upload_task::{NewUploadTask, UpdateUploadTask, UploadTask};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error as DieselError;

#[derive(Clone)]
pub struct UploadTaskRepository {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl UploadTaskRepository {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new_task: NewUploadTask) -> Result<UploadTask, DieselError> {
        use glance_mind_db::schema::gm_upload_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        diesel::insert_into(gm_upload_tasks)
            .values(&new_task)
            .returning(UploadTask::as_select())
            .get_result(&mut conn)
    }

    pub async fn find_by_id(&self, task_id: i32) -> Result<UploadTask, DieselError> {
        use glance_mind_db::schema::gm_upload_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        gm_upload_tasks
            .select(UploadTask::as_select())
            .filter(id.eq(task_id))
            .first(&mut conn)
    }

    pub async fn find_by_device_and_status(
        &self,
        device_id_param: String,
        status_filter: Option<String>,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<UploadTask>, i64), DieselError> {
        use glance_mind_db::schema::{gm_social_accounts, gm_upload_tasks};

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        // Build base query for count
        let count_query = gm_upload_tasks::table
            .inner_join(
                gm_social_accounts::table
                    .on(gm_upload_tasks::social_account_id.eq(gm_social_accounts::id)),
            )
            .filter(gm_social_accounts::device_id.eq(&device_id_param));

        // Get total count
        let total = if let Some(ref status_val) = status_filter {
            count_query
                .filter(gm_upload_tasks::status.eq(status_val))
                .select(diesel::dsl::count(gm_upload_tasks::id))
                .first::<i64>(&mut conn)?
        } else {
            count_query
                .select(diesel::dsl::count(gm_upload_tasks::id))
                .first::<i64>(&mut conn)?
        };

        // Build query for paginated results
        let mut tasks_query = gm_upload_tasks::table
            .inner_join(
                gm_social_accounts::table
                    .on(gm_upload_tasks::social_account_id.eq(gm_social_accounts::id)),
            )
            .filter(gm_social_accounts::device_id.eq(&device_id_param))
            .into_boxed();

        // Apply status filter if provided
        if let Some(status_val) = status_filter {
            tasks_query = tasks_query.filter(gm_upload_tasks::status.eq(status_val));
        }

        // Get paginated results
        let tasks = tasks_query
            .select(UploadTask::as_select())
            .order(gm_upload_tasks::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .load::<UploadTask>(&mut conn)?;

        Ok((tasks, total))
    }

    pub async fn find_by_user_paginated(
        &self,
        user_id_param: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<UploadTask>, i64), DieselError> {
        use glance_mind_db::schema::gm_upload_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        // Get total count
        let total = gm_upload_tasks
            .filter(user_id.eq(user_id_param))
            .select(diesel::dsl::count(id))
            .first::<i64>(&mut conn)?;

        // Get paginated results
        let tasks = gm_upload_tasks
            .filter(user_id.eq(user_id_param))
            .order(created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(UploadTask::as_select())
            .load(&mut conn)?;

        Ok((tasks, total))
    }

    pub async fn find_by_user_with_filter(
        &self,
        user_id_param: i32,
        page: i64,
        page_size: i64,
        status_filter: Option<String>,
        platform_id_filter: Option<i32>,
    ) -> Result<(Vec<UploadTask>, i64), DieselError> {
        use glance_mind_db::schema::gm_upload_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        // Build base query
        let mut count_query = gm_upload_tasks
            .filter(user_id.eq(user_id_param))
            .into_boxed();

        // Apply status filter
        if let Some(ref status_val) = status_filter {
            count_query = count_query.filter(status.eq(status_val));
        }

        // Apply platform_id filter
        if let Some(platform_id_val) = platform_id_filter {
            count_query = count_query.filter(platform_id.eq(platform_id_val));
        }

        // Get total count
        let total = count_query
            .select(diesel::dsl::count(id))
            .first::<i64>(&mut conn)?;

        // Build query for paginated results
        let mut tasks_query = gm_upload_tasks
            .filter(user_id.eq(user_id_param))
            .into_boxed();

        // Apply status filter
        if let Some(status_val) = status_filter {
            tasks_query = tasks_query.filter(status.eq(status_val));
        }

        // Apply platform_id filter
        if let Some(platform_id_val) = platform_id_filter {
            tasks_query = tasks_query.filter(platform_id.eq(platform_id_val));
        }

        // Get paginated results
        let tasks = tasks_query
            .order(created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(UploadTask::as_select())
            .load(&mut conn)?;

        Ok((tasks, total))
    }

    pub async fn update(
        &self,
        task_id: i32,
        update: UpdateUploadTask,
    ) -> Result<UploadTask, DieselError> {
        use glance_mind_db::schema::gm_upload_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        diesel::update(gm_upload_tasks.filter(id.eq(task_id)))
            .set(&update)
            .returning(UploadTask::as_select())
            .get_result(&mut conn)
    }
}
