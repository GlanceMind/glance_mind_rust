use crate::error::api_error::ApiError;
use chrono::Utc;
use diesel::prelude::*;
use glance_mind_db::entity::ai_chat::*;
use glance_mind_db::schema::*;
use std::sync::Arc;

#[derive(Clone)]
pub struct AiChatRepository {
    db: Arc<crate::config::database::Database>,
}

impl AiChatRepository {
    pub fn new(db: Arc<crate::config::database::Database>) -> Self {
        Self { db }
    }

    fn conn(&self) -> Result<diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<PgConnection>>, ApiError> {
        self.db.pool.get().map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    // ── Conversations ───────────────────────────────────────

    pub fn create_conversation(&self, user_id: i32, title: &str) -> Result<AiConversation, ApiError> {
        let new = NewAiConversation {
            user_id,
            title: title.to_string(),
            status: "active".to_string(),
        };
        diesel::insert_into(gm_ai_conversations::table)
            .values(&new)
            .get_result::<AiConversation>(&mut *self.conn()?)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    pub fn list_conversations(&self, user_id: i32, page: i32, page_size: i32) -> Result<(Vec<AiConversation>, i64), ApiError> {
        let mut conn = self.conn()?;
        let offset = ((page - 1) * page_size) as i64;

        let total: i64 = gm_ai_conversations::table
            .filter(gm_ai_conversations::user_id.eq(user_id))
            .filter(gm_ai_conversations::status.eq("active"))
            .count()
            .get_result(&mut *conn)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let items = gm_ai_conversations::table
            .filter(gm_ai_conversations::user_id.eq(user_id))
            .filter(gm_ai_conversations::status.eq("active"))
            .order(gm_ai_conversations::updated_at.desc().nulls_last())
            .then_order_by(gm_ai_conversations::created_at.desc())
            .offset(offset)
            .limit(page_size as i64)
            .load::<AiConversation>(&mut *conn)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok((items, total))
    }

    pub fn get_conversation(&self, id: i32, user_id: i32) -> Result<Option<AiConversation>, ApiError> {
        gm_ai_conversations::table
            .filter(gm_ai_conversations::id.eq(id))
            .filter(gm_ai_conversations::user_id.eq(user_id))
            .first::<AiConversation>(&mut *self.conn()?)
            .optional()
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    pub fn update_conversation(&self, id: i32, user_id: i32, update: &UpdateAiConversation) -> Result<AiConversation, ApiError> {
        diesel::update(
            gm_ai_conversations::table
                .filter(gm_ai_conversations::id.eq(id))
                .filter(gm_ai_conversations::user_id.eq(user_id)),
        )
        .set(update)
        .get_result::<AiConversation>(&mut *self.conn()?)
        .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    pub fn delete_conversation(&self, id: i32, user_id: i32) -> Result<usize, ApiError> {
        diesel::delete(
            gm_ai_conversations::table
                .filter(gm_ai_conversations::id.eq(id))
                .filter(gm_ai_conversations::user_id.eq(user_id)),
        )
        .execute(&mut *self.conn()?)
        .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    // ── Messages ────────────────────────────────────────────

    pub fn create_message(&self, new: &NewAiMessage) -> Result<AiMessage, ApiError> {
        diesel::insert_into(gm_ai_messages::table)
            .values(new)
            .get_result::<AiMessage>(&mut *self.conn()?)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    pub fn list_messages(&self, conversation_id: i32, limit: i64) -> Result<Vec<AiMessage>, ApiError> {
        gm_ai_messages::table
            .filter(gm_ai_messages::conversation_id.eq(conversation_id))
            .order(gm_ai_messages::created_at.asc())
            .limit(limit)
            .load::<AiMessage>(&mut *self.conn()?)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    // ── Plans ───────────────────────────────────────────────

    pub fn create_plan(&self, new: &NewAiPlan) -> Result<AiPlan, ApiError> {
        diesel::insert_into(gm_ai_plans::table)
            .values(new)
            .get_result::<AiPlan>(&mut *self.conn()?)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    pub fn get_plan(&self, id: i32, user_id: i32) -> Result<Option<AiPlan>, ApiError> {
        gm_ai_plans::table
            .filter(gm_ai_plans::id.eq(id))
            .filter(gm_ai_plans::user_id.eq(user_id))
            .first::<AiPlan>(&mut *self.conn()?)
            .optional()
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    pub fn update_plan(&self, id: i32, user_id: i32, update: &UpdateAiPlan) -> Result<AiPlan, ApiError> {
        diesel::update(
            gm_ai_plans::table
                .filter(gm_ai_plans::id.eq(id))
                .filter(gm_ai_plans::user_id.eq(user_id)),
        )
        .set(update)
        .get_result::<AiPlan>(&mut *self.conn()?)
        .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    // ── Plan Steps ──────────────────────────────────────────

    pub fn create_plan_steps(&self, steps: &[NewAiPlanStep]) -> Result<Vec<AiPlanStep>, ApiError> {
        diesel::insert_into(gm_ai_plan_steps::table)
            .values(steps)
            .get_results::<AiPlanStep>(&mut *self.conn()?)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    pub fn get_plan_steps(&self, plan_id: i32) -> Result<Vec<AiPlanStep>, ApiError> {
        gm_ai_plan_steps::table
            .filter(gm_ai_plan_steps::plan_id.eq(plan_id))
            .order(gm_ai_plan_steps::step_order.asc())
            .load::<AiPlanStep>(&mut *self.conn()?)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    pub fn update_plan_step(&self, id: i32, update: &UpdateAiPlanStep) -> Result<AiPlanStep, ApiError> {
        diesel::update(gm_ai_plan_steps::table.filter(gm_ai_plan_steps::id.eq(id)))
            .set(update)
            .get_result::<AiPlanStep>(&mut *self.conn()?)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    // ── Audit Log ───────────────────────────────────────────

    pub fn log_tool_call(&self, new: &NewAiToolAuditLog) -> Result<(), ApiError> {
        diesel::insert_into(gm_ai_tool_audit_logs::table)
            .values(new)
            .execute(&mut *self.conn()?)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub fn touch_conversation(&self, id: i32) -> Result<(), ApiError> {
        diesel::update(gm_ai_conversations::table.filter(gm_ai_conversations::id.eq(id)))
            .set(gm_ai_conversations::updated_at.eq(Some(Utc::now())))
            .execute(&mut *self.conn()?)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
        Ok(())
    }
}
