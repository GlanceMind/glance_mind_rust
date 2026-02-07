//! AI Publish Module Repository
//! 自动发布模块数据访问层

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error as DieselError;
use glance_mind_db::entity::aipub::{
    AiTaskStatus, AipubAiTask, AipubPlan, AipubTask, NewAipubAiTask, NewAipubPlan, NewAipubTask,
    PublishTaskStatus, UpdateAipubAiTask, UpdateAipubPlan, UpdateAipubTask,
};

#[derive(Clone)]
pub struct AipubRepository {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl AipubRepository {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    // =========================================================================
    // Plan Repository Methods
    // =========================================================================

    pub async fn create_plan(&self, new_plan: NewAipubPlan) -> Result<AipubPlan, DieselError> {
        use glance_mind_db::schema::gm_aipub_plans::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        diesel::insert_into(gm_aipub_plans)
            .values(&new_plan)
            .returning(AipubPlan::as_select())
            .get_result(&mut conn)
    }

    pub async fn find_plan_by_id(&self, plan_id: i32) -> Result<AipubPlan, DieselError> {
        use glance_mind_db::schema::gm_aipub_plans::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        gm_aipub_plans
            .select(AipubPlan::as_select())
            .filter(id.eq(plan_id))
            .first(&mut conn)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn find_plans_by_user(
        &self,
        user_id_param: i32,
        page: i64,
        page_size: i64,
        status_filter: Option<String>,
        platform_id_filter: Option<i32>,
        content_type_filter: Option<String>,
        plan_type_filter: Option<String>,
    ) -> Result<(Vec<AipubPlan>, i64), DieselError> {
        use glance_mind_db::schema::gm_aipub_plans::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        // Build count query
        let mut count_query = gm_aipub_plans
            .filter(user_id.eq(user_id_param))
            .into_boxed();

        if let Some(ref s) = status_filter {
            count_query = count_query.filter(status.eq(s));
        }
        if let Some(p) = platform_id_filter {
            count_query = count_query.filter(platform_id.eq(p));
        }
        if let Some(ref ct) = content_type_filter {
            count_query = count_query.filter(content_type.eq(ct));
        }
        if let Some(ref pt) = plan_type_filter {
            count_query = count_query.filter(plan_type.eq(pt));
        }

        let total = count_query
            .select(diesel::dsl::count(id))
            .first::<i64>(&mut conn)?;

        // Build paginated query
        let mut plans_query = gm_aipub_plans
            .filter(user_id.eq(user_id_param))
            .into_boxed();

        if let Some(s) = status_filter {
            plans_query = plans_query.filter(status.eq(s));
        }
        if let Some(p) = platform_id_filter {
            plans_query = plans_query.filter(platform_id.eq(p));
        }
        if let Some(ct) = content_type_filter {
            plans_query = plans_query.filter(content_type.eq(ct));
        }
        if let Some(pt) = plan_type_filter {
            plans_query = plans_query.filter(plan_type.eq(pt));
        }

        let plans = plans_query
            .select(AipubPlan::as_select())
            .order(created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .load(&mut conn)?;

        Ok((plans, total))
    }

    pub async fn update_plan(
        &self,
        plan_id: i32,
        update: UpdateAipubPlan,
    ) -> Result<AipubPlan, DieselError> {
        use glance_mind_db::schema::gm_aipub_plans::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        diesel::update(gm_aipub_plans.filter(id.eq(plan_id)))
            .set(&update)
            .returning(AipubPlan::as_select())
            .get_result(&mut conn)
    }

    pub async fn delete_plan(&self, plan_id: i32) -> Result<usize, DieselError> {
        use glance_mind_db::schema::gm_aipub_plans::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        diesel::delete(gm_aipub_plans.filter(id.eq(plan_id))).execute(&mut conn)
    }

    pub async fn count_plans_by_status(
        &self,
        user_id_param: i32,
    ) -> Result<Vec<(String, i64)>, DieselError> {
        use glance_mind_db::schema::gm_aipub_plans::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        gm_aipub_plans
            .filter(user_id.eq(user_id_param))
            .group_by(status)
            .select((status, diesel::dsl::count(id)))
            .load::<(String, i64)>(&mut conn)
    }

    // =========================================================================
    // AI Task Repository Methods
    // =========================================================================

    pub async fn create_ai_task(
        &self,
        new_task: NewAipubAiTask,
    ) -> Result<AipubAiTask, DieselError> {
        use glance_mind_db::schema::gm_aipub_ai_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        diesel::insert_into(gm_aipub_ai_tasks)
            .values(&new_task)
            .returning(AipubAiTask::as_select())
            .get_result(&mut conn)
    }

    pub async fn find_ai_task_by_id(&self, task_id: i32) -> Result<AipubAiTask, DieselError> {
        use glance_mind_db::schema::gm_aipub_ai_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        gm_aipub_ai_tasks
            .select(AipubAiTask::as_select())
            .filter(id.eq(task_id))
            .first(&mut conn)
    }

    pub async fn find_ai_tasks_by_plan(
        &self,
        plan_id_param: i32,
    ) -> Result<Vec<AipubAiTask>, DieselError> {
        use glance_mind_db::schema::gm_aipub_ai_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        gm_aipub_ai_tasks
            .select(AipubAiTask::as_select())
            .filter(plan_id.eq(plan_id_param))
            .order(created_at.asc())
            .load(&mut conn)
    }

    pub async fn find_processing_ai_tasks(
        &self,
        limit_param: i64,
    ) -> Result<Vec<AipubAiTask>, DieselError> {
        use glance_mind_db::schema::gm_aipub_ai_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        gm_aipub_ai_tasks
            .select(AipubAiTask::as_select())
            .filter(status.eq(AiTaskStatus::Processing.as_str()))
            .order(created_at.asc())
            .limit(limit_param)
            .load(&mut conn)
    }

    pub async fn update_ai_task(
        &self,
        task_id: i32,
        update: UpdateAipubAiTask,
    ) -> Result<AipubAiTask, DieselError> {
        use glance_mind_db::schema::gm_aipub_ai_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        diesel::update(gm_aipub_ai_tasks.filter(id.eq(task_id)))
            .set(&update)
            .returning(AipubAiTask::as_select())
            .get_result(&mut conn)
    }

    pub async fn count_ai_tasks_by_plan_and_status(
        &self,
        plan_id_param: i32,
    ) -> Result<Vec<(String, i64)>, DieselError> {
        use glance_mind_db::schema::gm_aipub_ai_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        gm_aipub_ai_tasks
            .filter(plan_id.eq(plan_id_param))
            .group_by(status)
            .select((status, diesel::dsl::count(id)))
            .load::<(String, i64)>(&mut conn)
    }

    // =========================================================================
    // Publish Task Repository Methods
    // =========================================================================

    pub async fn create_publish_task(
        &self,
        new_task: NewAipubTask,
    ) -> Result<AipubTask, DieselError> {
        use glance_mind_db::schema::gm_aipub_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        diesel::insert_into(gm_aipub_tasks)
            .values(&new_task)
            .returning(AipubTask::as_select())
            .get_result(&mut conn)
    }

    pub async fn create_publish_tasks_batch(
        &self,
        new_tasks: Vec<NewAipubTask>,
    ) -> Result<Vec<AipubTask>, DieselError> {
        use glance_mind_db::schema::gm_aipub_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        diesel::insert_into(gm_aipub_tasks)
            .values(&new_tasks)
            .returning(AipubTask::as_select())
            .get_results(&mut conn)
    }

    pub async fn find_publish_task_by_id(&self, task_id: i32) -> Result<AipubTask, DieselError> {
        use glance_mind_db::schema::gm_aipub_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        gm_aipub_tasks
            .select(AipubTask::as_select())
            .filter(id.eq(task_id))
            .first(&mut conn)
    }

    pub async fn find_publish_tasks_by_plan(
        &self,
        plan_id_param: i32,
    ) -> Result<Vec<AipubTask>, DieselError> {
        use glance_mind_db::schema::gm_aipub_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        gm_aipub_tasks
            .select(AipubTask::as_select())
            .filter(plan_id.eq(plan_id_param))
            .order(created_at.asc())
            .load(&mut conn)
    }

    pub async fn find_ready_publish_tasks_by_device(
        &self,
        device_id_param: &str,
        platform_filter: Option<&str>,
        limit_param: i64,
    ) -> Result<Vec<(AipubTask, AipubPlan, String, Option<String>, i32)>, DieselError> {
        use glance_mind_db::schema::{
            gm_aipub_plans, gm_aipub_tasks, gm_platforms, gm_social_accounts,
        };

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        let mut query = gm_aipub_tasks::table
            .inner_join(gm_aipub_plans::table.on(gm_aipub_tasks::plan_id.eq(gm_aipub_plans::id)))
            .inner_join(
                gm_social_accounts::table
                    .on(gm_aipub_tasks::social_account_id.eq(gm_social_accounts::id)),
            )
            .inner_join(gm_platforms::table.on(gm_aipub_plans::platform_id.eq(gm_platforms::id)))
            .filter(gm_aipub_tasks::status.eq(PublishTaskStatus::Ready.as_str()))
            .filter(gm_social_accounts::device_id.eq(device_id_param))
            .into_boxed();

        if let Some(platform) = platform_filter {
            query = query.filter(gm_platforms::name.eq(platform));
        }

        query
            .select((
                AipubTask::as_select(),
                AipubPlan::as_select(),
                gm_platforms::name,
                gm_social_accounts::profile_name,
                gm_platforms::id,
            ))
            .order(gm_aipub_tasks::created_at.asc())
            .limit(limit_param)
            .load(&mut conn)
    }

    pub async fn update_publish_task(
        &self,
        task_id: i32,
        update: UpdateAipubTask,
    ) -> Result<AipubTask, DieselError> {
        use glance_mind_db::schema::gm_aipub_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        diesel::update(gm_aipub_tasks.filter(id.eq(task_id)))
            .set(&update)
            .returning(AipubTask::as_select())
            .get_result(&mut conn)
    }

    pub async fn count_publish_tasks_by_plan_and_status(
        &self,
        plan_id_param: i32,
    ) -> Result<Vec<(String, i64)>, DieselError> {
        use glance_mind_db::schema::gm_aipub_tasks::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        gm_aipub_tasks
            .filter(plan_id.eq(plan_id_param))
            .group_by(status)
            .select((status, diesel::dsl::count(id)))
            .load::<(String, i64)>(&mut conn)
    }

    /// Find all publish tasks for a user's plans
    pub async fn find_publish_tasks_by_user(
        &self,
        user_id_param: i32,
        status_filter: Option<String>,
        platform_filter: Option<i32>,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<(AipubTask, AipubPlan, String, Option<String>, i32)>, i64), DieselError> {
        use glance_mind_db::schema::{
            gm_aipub_plans, gm_aipub_tasks, gm_platforms, gm_social_accounts,
        };

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        // Base query with joins
        let mut query = gm_aipub_tasks::table
            .inner_join(gm_aipub_plans::table.on(gm_aipub_tasks::plan_id.eq(gm_aipub_plans::id)))
            .inner_join(
                gm_social_accounts::table
                    .on(gm_aipub_tasks::social_account_id.eq(gm_social_accounts::id)),
            )
            .inner_join(gm_platforms::table.on(gm_aipub_plans::platform_id.eq(gm_platforms::id)))
            .filter(gm_aipub_plans::user_id.eq(user_id_param))
            .into_boxed();

        // Apply status filter
        if let Some(ref s) = status_filter {
            query = query.filter(gm_aipub_tasks::status.eq(s));
        }

        // Apply platform filter
        if let Some(pid) = platform_filter {
            query = query.filter(gm_aipub_plans::platform_id.eq(pid));
        }

        // Get total count
        let count_query = gm_aipub_tasks::table
            .inner_join(gm_aipub_plans::table.on(gm_aipub_tasks::plan_id.eq(gm_aipub_plans::id)))
            .filter(gm_aipub_plans::user_id.eq(user_id_param));

        let total: i64 = if status_filter.is_some() || platform_filter.is_some() {
            let mut count_q = count_query.into_boxed();
            if let Some(ref s) = status_filter {
                count_q = count_q.filter(gm_aipub_tasks::status.eq(s));
            }
            if let Some(pid) = platform_filter {
                count_q = count_q.filter(gm_aipub_plans::platform_id.eq(pid));
            }
            count_q.count().get_result(&mut conn)?
        } else {
            count_query.count().get_result(&mut conn)?
        };

        // Get paginated results
        let offset = (page - 1) * page_size;
        let tasks = query
            .order(gm_aipub_tasks::created_at.desc())
            .offset(offset)
            .limit(page_size)
            .select((
                AipubTask::as_select(),
                AipubPlan::as_select(),
                gm_platforms::name,
                gm_social_accounts::profile_name.nullable(),
                gm_platforms::id,
            ))
            .load::<(AipubTask, AipubPlan, String, Option<String>, i32)>(&mut conn)?;

        Ok((tasks, total))
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    pub async fn get_group_account_ids(
        &self,
        group_id_param: i32,
    ) -> Result<Vec<i32>, DieselError> {
        use glance_mind_db::schema::gm_social_accounts::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        gm_social_accounts
            .filter(group_id.eq(group_id_param))
            .filter(status.eq("ACTIVE"))
            .select(id)
            .load(&mut conn)
    }

    pub fn get_platform_name(&self, platform_id_param: i32) -> Result<String, DieselError> {
        use glance_mind_db::schema::gm_platforms::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        gm_platforms
            .filter(id.eq(platform_id_param))
            .select(name)
            .first(&mut conn)
    }

    pub fn get_group_name(&self, group_id_param: i32) -> Result<String, DieselError> {
        use glance_mind_db::schema::gm_social_groups::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        gm_social_groups
            .filter(id.eq(group_id_param))
            .select(group_name)
            .first(&mut conn)
    }

    pub fn get_account_username(&self, account_id: i32) -> Result<String, DieselError> {
        use glance_mind_db::schema::gm_social_accounts::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        gm_social_accounts
            .filter(id.eq(account_id))
            .select(username)
            .first(&mut conn)
    }

    pub fn get_model_name(&self, model_id_param: i32) -> Result<String, DieselError> {
        use glance_mind_db::schema::gm_ai_models::dsl::*;

        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        gm_ai_models
            .filter(id.eq(model_id_param))
            .select(name)
            .first(&mut conn)
    }

    // =========================================================================
    // Billing Methods (call stored procedures)
    // =========================================================================

    /// Freeze estimated budget for a plan. Returns frozen amount.
    /// Stored procedure handles: idempotency, row locking, balance validation.
    pub async fn freeze_budget(
        &self,
        user_id: i32,
        chat_count: i32,
        image_count: i32,
        video_count: i32,
        chat_model_id: Option<i32>,
        image_model_id: Option<i32>,
        video_model_id: Option<i32>,
        ref_type: &str,
        ref_id: i32,
    ) -> Result<bigdecimal::BigDecimal, DieselError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        let result: bigdecimal::BigDecimal = diesel::sql_query(
            "SELECT fn_freeze_budget($1, $2, $3, $4, $5, $6, $7, $8, $9) as frozen_amount"
        )
        .bind::<diesel::sql_types::Integer, _>(user_id)
        .bind::<diesel::sql_types::Integer, _>(chat_count)
        .bind::<diesel::sql_types::Integer, _>(image_count)
        .bind::<diesel::sql_types::Integer, _>(video_count)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(chat_model_id)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(image_model_id)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(video_model_id)
        .bind::<diesel::sql_types::VarChar, _>(ref_type)
        .bind::<diesel::sql_types::Integer, _>(ref_id)
        .get_result::<FrozenAmountRow>(&mut conn)?
        .frozen_amount;

        Ok(result)
    }

    /// Finalize a plan: update status + refund remaining frozen.
    /// Stored procedure handles: idempotency, row locking, refund calculation.
    pub async fn finalize_plan(
        &self,
        plan_id: i32,
        new_status: &str,
    ) -> Result<(), DieselError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| DieselError::BrokenTransactionManager)?;

        diesel::sql_query("SELECT fn_finalize_plan($1, $2)")
            .bind::<diesel::sql_types::Integer, _>(plan_id)
            .bind::<diesel::sql_types::VarChar, _>(new_status)
            .execute(&mut conn)?;

        Ok(())
    }
}

/// Helper struct for reading fn_freeze_budget result
#[derive(diesel::QueryableByName)]
struct FrozenAmountRow {
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    frozen_amount: bigdecimal::BigDecimal,
}
