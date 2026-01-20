use crate::config::database::DBPool;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::SelectableHelper;
use glance_mind_db::entity::pricing_rule::PricingRule;
use glance_mind_db::schema::gm_pricing_rules as pricing_rules;

#[derive(Clone)]
pub struct PricingRepository {
    pool: DBPool,
}

impl PricingRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    pub async fn find_all(&self) -> Result<Vec<PricingRule>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        pricing_rules::table
            .select(PricingRule::as_select())
            .load(&mut conn)
    }

    pub async fn find_by_action_type(
        &self,
        action_type: &str,
        platform_id: Option<i32>,
    ) -> Result<Option<PricingRule>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");

        let mut query = pricing_rules::table
            .filter(pricing_rules::action_type.eq(action_type))
            .into_boxed();

        if let Some(pid) = platform_id {
            query = query.filter(pricing_rules::platform_id.eq(pid));
        } else {
            query = query.filter(pricing_rules::platform_id.is_null());
        }

        query
            .select(PricingRule::as_select())
            .first(&mut conn)
            .optional()
    }
}
