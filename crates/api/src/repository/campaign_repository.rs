use crate::config::database::DBPool;
use glance_mind_db::entity::campaign::{Campaign, NewCampaign};
use glance_mind_db::schema::gm_campaigns as campaigns;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::SelectableHelper;

#[derive(Clone)]
pub struct CampaignRepository {
    pool: DBPool,
}

impl CampaignRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new_campaign: NewCampaign) -> Result<Campaign, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::insert_into(campaigns::table)
            .values(&new_campaign)
            .returning(Campaign::as_returning())
            .get_result(&mut conn)
    }

    #[allow(dead_code)]
    pub async fn find_by_id(&self, id: i32) -> Result<Campaign, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        campaigns::table
            .find(id)
            .select(Campaign::as_select())
            .first(&mut conn)
    }

    pub async fn find_by_id_and_user(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<Campaign, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        campaigns::table
            .filter(campaigns::id.eq(id))
            .filter(campaigns::user_id.eq(user_id))
            .select(Campaign::as_select())
            .first(&mut conn)
    }

    pub async fn find_by_user(
        &self,
        user_id: i32,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<Campaign>, i64), DieselError> {
        let mut conn = self.pool.get().expect("Connection error");

        let total = campaigns::table
            .filter(campaigns::user_id.eq(user_id))
            .count()
            .get_result(&mut conn)?;

        let items = campaigns::table
            .filter(campaigns::user_id.eq(user_id))
            .order(campaigns::created_at.desc())
            .limit(page_size)
            .offset((page - 1) * page_size)
            .select(Campaign::as_select())
            .load(&mut conn)?;

        Ok((items, total))
    }

    pub async fn update(
        &self,
        id: i32,
        user_id: i32,
        changeset: &NewCampaign,
    ) -> Result<Campaign, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::update(campaigns::table)
            .filter(campaigns::id.eq(id))
            .filter(campaigns::user_id.eq(user_id))
            .set(changeset)
            .returning(Campaign::as_returning())
            .get_result(&mut conn)
    }

    pub async fn update_status(
        &self,
        id: i32,
        user_id: i32,
        new_status: &str,
    ) -> Result<Campaign, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::update(campaigns::table)
            .filter(campaigns::id.eq(id))
            .filter(campaigns::user_id.eq(user_id))
            .set(campaigns::status.eq(new_status))
            .returning(Campaign::as_returning())
            .get_result(&mut conn)
    }

    pub async fn update_status_and_freeze(
        &self,
        id: i32,
        user_id: i32,
        new_status: &str,
        is_frozen: bool,
    ) -> Result<Campaign, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::update(campaigns::table)
            .filter(campaigns::id.eq(id))
            .filter(campaigns::user_id.eq(user_id))
            .set((
                campaigns::status.eq(new_status),
                campaigns::is_frozen.eq(is_frozen),
            ))
            .returning(Campaign::as_returning())
            .get_result(&mut conn)
    }

    // Get count of videos (total scans) for a campaign
    pub async fn get_total_scans(&self, campaign_id: i32) -> Result<i64, DieselError> {
        use glance_mind_db::schema::{gm_agent_videos as agent_videos, gm_crawler_tasks as crawler_tasks};

        let mut conn = self.pool.get().expect("Connection error");

        agent_videos::table
            .inner_join(crawler_tasks::table)
            .filter(crawler_tasks::campaign_id.eq(campaign_id))
            .count()
            .get_result(&mut conn)
    }

    // Get count of AI-processed comments (ai replies) for a campaign
    pub async fn get_ai_replies_count(&self, campaign_id: i32) -> Result<i64, DieselError> {
        use glance_mind_db::schema::{
            gm_agent_comments as agent_comments, gm_agent_videos as agent_videos,
            gm_crawler_tasks as crawler_tasks,
        };

        let mut conn = self.pool.get().expect("Connection error");

        agent_comments::table
            .inner_join(agent_videos::table)
            .inner_join(crawler_tasks::table.on(agent_videos::task_id.eq(crawler_tasks::id)))
            .filter(crawler_tasks::campaign_id.eq(campaign_id))
            .count()
            .get_result(&mut conn)
    }
}
