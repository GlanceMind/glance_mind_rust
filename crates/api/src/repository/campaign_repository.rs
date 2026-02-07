use crate::config::database::DBPool;
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::sql_query;
use diesel::sql_types::{Bool, Integer, Numeric, Text};
use diesel::SelectableHelper;
use glance_mind_db::entity::campaign::{Campaign, NewCampaign};
use glance_mind_db::schema::gm_campaigns as campaigns;

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

    #[allow(dead_code)]
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
        use glance_mind_db::schema::{
            gm_agent_videos as agent_videos, gm_crawler_tasks as crawler_tasks,
        };

        let mut conn = self.pool.get().expect("Connection error");

        agent_videos::table
            .inner_join(crawler_tasks::table)
            .filter(crawler_tasks::campaign_id.eq(campaign_id))
            .count()
            .get_result(&mut conn)
    }

    // Get count of AI-processed comments (ai replies) for a campaign
    // Supports all platforms: TikTok, Instagram, Facebook, Twitter, Reddit
    pub async fn get_ai_replies_count(&self, campaign_id: i32) -> Result<i64, DieselError> {
        use glance_mind_db::schema::{
            gm_agent_comments as agent_comments,
            gm_agent_videos as agent_videos,
            gm_crawler_tasks as crawler_tasks,
            gm_agent_instagram_comments,
            gm_agent_facebook_comments,
            gm_agent_twitter_comments,
            gm_agent_reddit_comments,
        };

        let mut conn = self.pool.get().expect("Connection error");

        // TikTok comments (requires join through agent_videos and crawler_tasks)
        let tiktok_count: i64 = agent_comments::table
            .inner_join(agent_videos::table)
            .inner_join(crawler_tasks::table.on(agent_videos::task_id.eq(crawler_tasks::id)))
            .filter(crawler_tasks::campaign_id.eq(campaign_id))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        // Instagram comments (direct campaign_id relation)
        let instagram_count: i64 = gm_agent_instagram_comments::table
            .filter(gm_agent_instagram_comments::campaign_id.eq(campaign_id))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        // Facebook comments (direct campaign_id relation)
        let facebook_count: i64 = gm_agent_facebook_comments::table
            .filter(gm_agent_facebook_comments::campaign_id.eq(campaign_id))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        // Twitter comments (direct campaign_id relation)
        let twitter_count: i64 = gm_agent_twitter_comments::table
            .filter(gm_agent_twitter_comments::campaign_id.eq(campaign_id))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        // Reddit comments (direct campaign_id relation)
        let reddit_count: i64 = gm_agent_reddit_comments::table
            .filter(gm_agent_reddit_comments::campaign_id.eq(campaign_id))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        // Return sum of all platforms
        Ok(tiktok_count + instagram_count + facebook_count + twitter_count + reddit_count)
    }

    /// Activate campaign using stored procedure
    /// This atomically: validates balance, freezes budget, updates status, generates transaction log
    pub async fn activate_campaign(
        &self,
        campaign_id: i32,
    ) -> Result<ActivateCampaignResult, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        let result: ActivateCampaignResult = sql_query(
            r#"
            SELECT success, message
            FROM fn_activate_campaign($1)
            "#,
        )
        .bind::<Integer, _>(campaign_id)
        .get_result(&mut conn)
        .map_err(|e| format!("Failed to call fn_activate_campaign: {}", e))?;

        Ok(result)
    }

    /// Stop campaign gracefully using stored procedure
    pub async fn stop_gracefully(&self, campaign_id: i32) -> Result<StopCampaignResult, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;

        let result: StopCampaignResult = sql_query(
            r#"
            SELECT success, immediate_stopped, refunded_amount
            FROM fn_stop_campaign_gracefully($1)
            "#,
        )
        .bind::<Integer, _>(campaign_id)
        .get_result(&mut conn)
        .map_err(|e| format!("Failed to call fn_stop_campaign_gracefully: {}", e))?;

        Ok(result)
    }
}

/// Result from fn_activate_campaign stored procedure
#[derive(Debug, QueryableByName)]
pub struct ActivateCampaignResult {
    #[diesel(sql_type = Bool)]
    pub success: bool,
    #[diesel(sql_type = Text)]
    pub message: String,
}

/// Result from fn_stop_campaign_gracefully stored procedure
#[derive(Debug, QueryableByName)]
pub struct StopCampaignResult {
    #[diesel(sql_type = Bool)]
    pub success: bool,
    #[diesel(sql_type = Bool)]
    pub immediate_stopped: bool,
    #[diesel(sql_type = Numeric)]
    pub refunded_amount: BigDecimal,
}
