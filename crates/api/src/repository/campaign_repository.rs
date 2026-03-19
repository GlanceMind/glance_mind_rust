use crate::config::database::DBPool;
use crate::platform_routing::SupportedPlatform;
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

    fn get_campaign_platform(
        &self,
        conn: &mut PgConnection,
        campaign_id: i32,
    ) -> Result<SupportedPlatform, DieselError> {
        use glance_mind_db::schema::{gm_campaigns, gm_platforms};

        let platform_name: String = gm_campaigns::table
            .inner_join(gm_platforms::table.on(gm_campaigns::platform_id.eq(gm_platforms::id)))
            .filter(gm_campaigns::id.eq(campaign_id))
            .select(gm_platforms::name)
            .first(conn)?;

        SupportedPlatform::from_name(&platform_name).ok_or(DieselError::NotFound)
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
            gm_agent_facebook_posts, gm_agent_instagram_posts, gm_agent_reddit_posts,
            gm_agent_twitter_tweets, gm_agent_videos,
        };

        let mut conn = self.pool.get().expect("Connection error");
        let platform = self.get_campaign_platform(&mut conn, campaign_id)?;

        match platform {
            SupportedPlatform::Tiktok => gm_agent_videos::table
                .filter(gm_agent_videos::campaign_id.eq(Some(campaign_id)))
                .count()
                .get_result(&mut conn),
            SupportedPlatform::Facebook => gm_agent_facebook_posts::table
                .filter(gm_agent_facebook_posts::campaign_id.eq(Some(campaign_id)))
                .count()
                .get_result(&mut conn),
            SupportedPlatform::Instagram => gm_agent_instagram_posts::table
                .filter(gm_agent_instagram_posts::campaign_id.eq(Some(campaign_id)))
                .count()
                .get_result(&mut conn),
            SupportedPlatform::Reddit => gm_agent_reddit_posts::table
                .filter(gm_agent_reddit_posts::campaign_id.eq(Some(campaign_id)))
                .count()
                .get_result(&mut conn),
            SupportedPlatform::Twitter => gm_agent_twitter_tweets::table
                .filter(gm_agent_twitter_tweets::campaign_id.eq(Some(campaign_id)))
                .count()
                .get_result(&mut conn),
        }
    }

    // Get count of AI-processed comments (ai replies) for a campaign
    // Supports all platforms: TikTok, Instagram, Facebook, Twitter, Reddit
    pub async fn get_ai_replies_count(&self, campaign_id: i32) -> Result<i64, DieselError> {
        use glance_mind_db::schema::{
            gm_agent_comments, gm_agent_facebook_comments, gm_agent_instagram_comments,
            gm_agent_reddit_comments, gm_agent_twitter_comments,
        };

        let mut conn = self.pool.get().expect("Connection error");
        let platform = self.get_campaign_platform(&mut conn, campaign_id)?;

        match platform {
            SupportedPlatform::Tiktok => gm_agent_comments::table
                .filter(gm_agent_comments::campaign_id.eq(Some(campaign_id)))
                .count()
                .get_result(&mut conn),
            SupportedPlatform::Facebook => gm_agent_facebook_comments::table
                .filter(gm_agent_facebook_comments::campaign_id.eq(Some(campaign_id)))
                .count()
                .get_result(&mut conn),
            SupportedPlatform::Instagram => gm_agent_instagram_comments::table
                .filter(gm_agent_instagram_comments::campaign_id.eq(Some(campaign_id)))
                .count()
                .get_result(&mut conn),
            SupportedPlatform::Reddit => gm_agent_reddit_comments::table
                .filter(gm_agent_reddit_comments::campaign_id.eq(Some(campaign_id)))
                .count()
                .get_result(&mut conn),
            SupportedPlatform::Twitter => gm_agent_twitter_comments::table
                .filter(gm_agent_twitter_comments::campaign_id.eq(Some(campaign_id)))
                .count()
                .get_result(&mut conn),
        }
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
