use crate::config::database::Database;
use crate::repository::ai_model_repository::AiModelRepository;
use crate::repository::pricing_repository::PricingRepository;
use diesel::result::Error as DieselError;
use glance_mind_db::entity::ai_model::AiModel;
use glance_mind_db::entity::pricing_rule::PricingRule;
use std::sync::Arc;

#[derive(Clone)]
pub struct ConfigService {
    ai_model_repo: AiModelRepository,
    pricing_repo: PricingRepository,
}

impl ConfigService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            ai_model_repo: AiModelRepository::new(db.pool.clone()),
            pricing_repo: PricingRepository::new(db.pool.clone()),
        }
    }

    pub async fn get_active_ai_models(&self) -> Result<Vec<AiModel>, DieselError> {
        self.ai_model_repo.find_all_active().await
    }

    pub async fn get_ai_models_by_type(
        &self,
        model_type: &str,
    ) -> Result<Vec<AiModel>, DieselError> {
        self.ai_model_repo.find_by_type(model_type).await
    }

    pub async fn get_ai_model_by_id(&self, id: i32) -> Result<Option<AiModel>, DieselError> {
        self.ai_model_repo.find_by_id(id).await
    }

    pub async fn get_pricing_rules(&self) -> Result<Vec<PricingRule>, DieselError> {
        self.pricing_repo.find_all().await
    }
}
