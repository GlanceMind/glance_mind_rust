use crate::config::database::DBPool;
use glance_mind_db::entity::ai_model::AiModel;
use glance_mind_db::schema::gm_ai_models as ai_models;
use diesel::prelude::*;
use diesel::result::Error as DieselError;

#[derive(Clone)]
pub struct AiModelRepository {
    pool: DBPool,
}

impl AiModelRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    /// Get all active models
    pub async fn find_all_active(&self) -> Result<Vec<AiModel>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        ai_models::table
            .filter(ai_models::is_active.eq(true))
            .order(ai_models::created_at.asc())
            .load(&mut conn)
    }

    /// Get active models by type
    pub async fn find_by_type(&self, model_type: &str) -> Result<Vec<AiModel>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        ai_models::table
            .filter(ai_models::is_active.eq(true))
            .filter(ai_models::model_type.eq(model_type))
            .order(ai_models::created_at.asc())
            .load(&mut conn)
    }

    /// Get model by ID
    pub async fn find_by_id(&self, id: i32) -> Result<Option<AiModel>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        ai_models::table.find(id).first(&mut conn).optional()
    }
}
