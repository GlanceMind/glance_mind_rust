use crate::config::database::DBPool;
use crate::platform_routing::SupportedPlatform;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::sql_query;
use diesel::sql_types::{
    Array, BigInt, Bool, Integer, Jsonb, Nullable, Numeric, Text, Timestamptz,
};
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

    pub async fn create_with_reply_template_ids(
        &self,
        new_campaign: NewCampaign,
        reply_template_ids: &[i32],
    ) -> Result<Campaign, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        conn.transaction(|conn| {
            Self::lock_reply_template_ids_for_user(conn, new_campaign.user_id, reply_template_ids)?;

            diesel::insert_into(campaigns::table)
                .values(&new_campaign)
                .returning(Campaign::as_returning())
                .get_result(conn)
        })
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
        conn.transaction(|conn| {
            diesel::sql_query(
                r#"
                UPDATE gm_campaigns
                SET
                    user_id = $3,
                    name = $4,
                    status = $5,
                    target_audience = $6,
                    keyword = $7,
                    schedule_config = $8,
                    platform_id = $9,
                    region_id = $10,
                    enable_ai_refactor = $11,
                    ai_model_id = $12,
                    persona_id = $13,
                    max_scan_count = $14,
                    budget_cap = $15,
                    end_date = $16,
                    schedule_type = $17,
                    product_prompt = $18,
                    call_to_action = $19,
                    tone_of_voice = $20,
                    additional_info = $21,
                    social_group_id = $22,
                    auto_like = $23,
                    auto_follow = $24,
                    auto_dm = $25,
                    auto_reply_comments = $26,
                    auto_reply_post = $27,
                    search_options = $28
                WHERE id = $1
                  AND user_id = $2
                "#,
            )
            .bind::<Integer, _>(id)
            .bind::<Integer, _>(user_id)
            .bind::<Integer, _>(changeset.user_id)
            .bind::<Text, _>(&changeset.name)
            .bind::<Nullable<Text>, _>(&changeset.status)
            .bind::<Nullable<Text>, _>(&changeset.target_audience)
            .bind::<Nullable<Text>, _>(&changeset.keyword)
            .bind::<Nullable<Jsonb>, _>(&changeset.schedule_config)
            .bind::<Integer, _>(changeset.platform_id)
            .bind::<Integer, _>(changeset.region_id)
            .bind::<Nullable<Bool>, _>(changeset.enable_ai_refactor)
            .bind::<Integer, _>(changeset.ai_model_id)
            .bind::<Nullable<Integer>, _>(changeset.persona_id)
            .bind::<Nullable<Integer>, _>(changeset.max_scan_count)
            .bind::<Nullable<Numeric>, _>(&changeset.budget_cap)
            .bind::<Nullable<Timestamptz>, _>(changeset.end_date)
            .bind::<Text, _>(&changeset.schedule_type)
            .bind::<Text, _>(&changeset.product_prompt)
            .bind::<Nullable<Text>, _>(&changeset.call_to_action)
            .bind::<Nullable<Text>, _>(&changeset.tone_of_voice)
            .bind::<Nullable<Text>, _>(&changeset.additional_info)
            .bind::<Nullable<Integer>, _>(changeset.social_group_id)
            .bind::<Nullable<Bool>, _>(changeset.auto_like)
            .bind::<Nullable<Bool>, _>(changeset.auto_follow)
            .bind::<Nullable<Bool>, _>(changeset.auto_dm)
            .bind::<Nullable<Bool>, _>(changeset.auto_reply_comments)
            .bind::<Nullable<Bool>, _>(changeset.auto_reply_post)
            .bind::<Nullable<Jsonb>, _>(&changeset.search_options)
            .execute(conn)?;

            campaigns::table
                .filter(campaigns::id.eq(id))
                .filter(campaigns::user_id.eq(user_id))
                .select(Campaign::as_select())
                .first(conn)
        })
    }

    pub async fn update_with_reply_template_ids(
        &self,
        id: i32,
        user_id: i32,
        changeset: &NewCampaign,
        reply_template_ids: &[i32],
    ) -> Result<Campaign, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        conn.transaction(|conn| {
            campaigns::table
                .filter(campaigns::id.eq(id))
                .filter(campaigns::user_id.eq(user_id))
                .for_update()
                .select(campaigns::id)
                .first::<i32>(conn)?;

            Self::lock_reply_template_ids_for_user(conn, user_id, reply_template_ids)?;

            let updated = diesel::update(campaigns::table)
                .filter(campaigns::id.eq(id))
                .filter(campaigns::user_id.eq(user_id))
                .set(changeset)
                .returning(Campaign::as_returning())
                .get_result(conn)?;

            diesel::sql_query(
                r#"
                DELETE FROM gm_campaign_templates
                WHERE campaign_id = $1
                  AND library_template_id IS NOT NULL
                  AND NOT (library_template_id = ANY($2))
                "#,
            )
            .bind::<Integer, _>(id)
            .bind::<Array<Integer>, _>(reply_template_ids)
            .execute(conn)?;

            Ok(updated)
        })
    }

    fn lock_reply_template_ids_for_user(
        conn: &mut PgConnection,
        user_id: i32,
        reply_template_ids: &[i32],
    ) -> Result<(), DieselError> {
        if reply_template_ids.is_empty() {
            return Ok(());
        }

        let locked_template_ids = diesel::sql_query(
            r#"
            SELECT id
            FROM gm_reply_template_library
            WHERE user_id = $1
              AND id = ANY($2)
            FOR KEY SHARE
            "#,
        )
        .bind::<Integer, _>(user_id)
        .bind::<Array<Integer>, _>(reply_template_ids)
        .load::<IdRow>(conn)?;

        let locked_template_id_set: std::collections::HashSet<i32> =
            locked_template_ids.iter().map(|row| row.id).collect();
        if locked_template_id_set.len() != reply_template_ids.len()
            || reply_template_ids
                .iter()
                .any(|id| !locked_template_id_set.contains(id))
        {
            return Err(DieselError::NotFound);
        }

        Ok(())
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

    /// Aggregate lead metrics (engagement feedback) for a campaign within a time window.
    ///
    /// **Branch B implementation** (see Task 0 findings 2026-04-17):
    /// the patrol writer is not idempotent for `gm_patrol_account_stats`, but the
    /// upstream-generated `(report_id, social_account_id)` pair is a stable business key.
    /// We dedup via `DISTINCT ON (report_id, social_account_id)` before summing.
    ///
    /// Only `report_type = 'notification'` rows are aggregated (incremental data);
    /// `'profile'` rows are cumulative snapshots and MUST NOT be summed.
    ///
    /// Returns: `(account_count, tracked_account_count, new_followers, dms,
    ///           friend_requests, mentions, last_updated_at)`
    ///
    /// `tracked_account_count` is clamped by `min(tracked_raw, account_count)` at the
    /// Rust layer to prevent concurrent-read inconsistency (D5).
    pub async fn aggregate_lead_metrics(
        &self,
        campaign_id: i32,
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
    ) -> Result<LeadMetricsAggregate, DieselError> {
        use glance_mind_db::schema::gm_campaign_accounts;

        let mut conn = self.pool.get().expect("Connection error");

        // Campaign account count (hard-delete only — schema has no deleted_at; see Task 0.7.2)
        let account_count: i64 = gm_campaign_accounts::table
            .filter(gm_campaign_accounts::campaign_id.eq(campaign_id))
            .count()
            .get_result(&mut conn)?;

        // Branch B: dedup by (report_id, social_account_id) then aggregate
        let row: LeadMetricsRow = sql_query(
            r#"
            WITH dedup AS (
                SELECT DISTINCT ON (s.report_id, s.social_account_id)
                    s.social_account_id,
                    s.new_followers,
                    s.received_dms,
                    s.received_friend_requests,
                    s.received_mentions,
                    s.received_likes,
                    s.received_comments,
                    s.collected_at
                FROM gm_patrol_account_stats s
                INNER JOIN gm_campaign_accounts ca
                    ON ca.account_id = s.social_account_id
                WHERE ca.campaign_id = $1
                  AND s.report_type = 'notification'
                  AND s.collected_at >= $2
                  AND s.collected_at <= $3
                ORDER BY s.report_id, s.social_account_id, s.id
            )
            SELECT
                COALESCE(SUM(new_followers), 0)::BIGINT           AS new_followers,
                COALESCE(SUM(received_dms), 0)::BIGINT            AS dms,
                COALESCE(SUM(received_friend_requests), 0)::BIGINT AS friend_requests,
                COALESCE(SUM(received_mentions), 0)::BIGINT       AS mentions,
                COALESCE(SUM(received_likes), 0)::BIGINT          AS received_likes,
                COALESCE(SUM(received_comments), 0)::BIGINT       AS received_comments,
                COUNT(DISTINCT social_account_id)::BIGINT         AS tracked_raw,
                MAX(collected_at)                                  AS last_updated_at
            FROM dedup
            "#,
        )
        .bind::<Integer, _>(campaign_id)
        .bind::<Timestamptz, _>(window_start)
        .bind::<Timestamptz, _>(window_end)
        .get_result(&mut conn)?;

        // D5 fix: clamp tracked to [0, account_count]
        let tracked_account_count = std::cmp::min(row.tracked_raw, account_count);

        Ok(LeadMetricsAggregate {
            account_count,
            tracked_account_count,
            new_followers: row.new_followers,
            dms: row.dms,
            friend_requests: row.friend_requests,
            mentions: row.mentions,
            received_likes: row.received_likes,
            received_comments: row.received_comments,
            last_updated_at: row.last_updated_at,
        })
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

#[derive(Debug, QueryableByName)]
struct IdRow {
    #[diesel(sql_type = Integer)]
    id: i32,
}

/// Raw row returned by the dedup-then-aggregate SQL in `aggregate_lead_metrics`.
#[derive(Debug, QueryableByName)]
struct LeadMetricsRow {
    #[diesel(sql_type = BigInt)]
    new_followers: i64,
    #[diesel(sql_type = BigInt)]
    dms: i64,
    #[diesel(sql_type = BigInt)]
    friend_requests: i64,
    #[diesel(sql_type = BigInt)]
    mentions: i64,
    #[diesel(sql_type = BigInt)]
    received_likes: i64,
    #[diesel(sql_type = BigInt)]
    received_comments: i64,
    #[diesel(sql_type = BigInt)]
    tracked_raw: i64,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    last_updated_at: Option<DateTime<Utc>>,
}

/// Cleaned / clamped aggregate returned to the service layer.
#[derive(Debug, Clone)]
pub struct LeadMetricsAggregate {
    pub account_count: i64,
    pub tracked_account_count: i64,
    pub new_followers: i64,
    pub dms: i64,
    pub friend_requests: i64,
    pub mentions: i64,
    pub received_likes: i64,
    pub received_comments: i64,
    pub last_updated_at: Option<DateTime<Utc>>,
}
