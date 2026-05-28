use crate::config::database::DBPool;
use crate::dto::dm_auto_reply_dto::{ReplyLogUpsertRequest, ReplyLogUpsertResponse};
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use glance_mind_db::entity::dm_auto_reply::{
    AutoReplyConfig, DmConversationReview, DmReplyLog, NewDmConversationReview, NewDmReplyLog,
    ProductFaq,
};
use glance_mind_db::schema::{
    gm_auto_reply_config, gm_campaign_accounts, gm_campaigns, gm_dm_conversation_review,
    gm_dm_reply_log, gm_product_faq,
};

pub const REPLY_ACTION_CREATED: &str = "created";
pub const REPLY_ACTION_UPDATED: &str = "updated";
pub const REPLY_ACTION_NOOP: &str = "noop";

const TERMINAL_STATUSES: &[&str] = &["sent", "escalated", "failed"];

/// Subset of `gm_campaigns` used by DM AI customer service to populate
/// `DmConfigResponse.product_info` / `dm_prompt`. §7.1.2: `gm_campaigns`
/// has no dedicated `dm_prompt` column, so we reuse `product_prompt`.
#[derive(Debug, Clone)]
pub struct CampaignContext {
    pub product_info: Option<String>,
    pub dm_prompt: Option<String>,
}

#[derive(Clone)]
pub struct DmAutoReplyRepository {
    pool: DBPool,
}

impl DmAutoReplyRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    pub fn get_auto_reply_config(
        &self,
        social_account_id: i32,
    ) -> Result<Option<AutoReplyConfig>, DieselError> {
        let mut conn = self.pool.get().expect("DB connection error");
        gm_auto_reply_config::table
            .filter(gm_auto_reply_config::social_account_id.eq(social_account_id))
            .select(AutoReplyConfig::as_select())
            .first::<AutoReplyConfig>(&mut conn)
            .optional()
    }

    /// Resolve the campaign that should drive this DM reply.
    ///
    /// Per §7.1 implementation findings, the planned `campaign_interactions` table
    /// does not exist; we fall back to the social account's most recently active
    /// campaign via `gm_campaign_accounts`.
    pub fn resolve_campaign_id(
        &self,
        social_account_id: i32,
        _user_ref: &str,
    ) -> Result<Option<i32>, DieselError> {
        let mut conn = self.pool.get().expect("DB connection error");
        gm_campaign_accounts::table
            .inner_join(
                gm_campaigns::table.on(gm_campaigns::id.eq(gm_campaign_accounts::campaign_id)),
            )
            .filter(gm_campaign_accounts::account_id.eq(social_account_id))
            .filter(gm_campaigns::status.eq("ACTIVE"))
            .order(gm_campaigns::created_at.desc())
            .select(gm_campaigns::id)
            .first::<i32>(&mut conn)
            .optional()
    }

    pub fn list_product_faqs(&self, campaign_id: i32) -> Result<Vec<ProductFaq>, DieselError> {
        let mut conn = self.pool.get().expect("DB connection error");
        gm_product_faq::table
            .filter(gm_product_faq::campaign_id.eq(campaign_id))
            .select(ProductFaq::as_select())
            .load::<ProductFaq>(&mut conn)
    }

    /// Fetch prompt columns used for Gateway→Dify context.
    /// Mapping (§7.1.2): `product_info`←`gm_campaigns.product_prompt`,
    /// `dm_prompt`←`gm_campaigns.additional_info` (falls back to
    /// `product_prompt` when unset; `gm_campaigns` has no dedicated
    /// `dm_prompt` column).
    pub fn get_campaign_context(
        &self,
        campaign_id: i32,
    ) -> Result<Option<CampaignContext>, DieselError> {
        let mut conn = self.pool.get().expect("DB connection error");
        let row: Option<(String, Option<String>)> = gm_campaigns::table
            .filter(gm_campaigns::id.eq(campaign_id))
            .select((gm_campaigns::product_prompt, gm_campaigns::additional_info))
            .first::<(String, Option<String>)>(&mut conn)
            .optional()?;
        Ok(row.map(|(product_prompt, additional_info)| {
            let product_info = if product_prompt.is_empty() {
                None
            } else {
                Some(product_prompt.clone())
            };
            let dm_prompt = additional_info
                .filter(|s| !s.is_empty())
                .or_else(|| product_info.clone());
            CampaignContext {
                product_info,
                dm_prompt,
            }
        }))
    }

    /// Count today's `sent` replies within the (campaign_id, user_ref)
    /// scope — the spec §5.4 rate-limit dimension.
    pub fn count_sent_replies_today(
        &self,
        campaign_id: i32,
        user_ref: &str,
    ) -> Result<i64, DieselError> {
        let mut conn = self.pool.get().expect("DB connection error");
        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|d| d.and_utc())
            .unwrap_or_else(Utc::now);
        gm_dm_reply_log::table
            .filter(gm_dm_reply_log::campaign_id.eq(Some(campaign_id)))
            .filter(gm_dm_reply_log::user_ref.eq(user_ref))
            .filter(gm_dm_reply_log::status.eq("sent"))
            .filter(gm_dm_reply_log::created_at.ge(today_start))
            .count()
            .get_result::<i64>(&mut conn)
    }

    /// Resolve the rate-limit ceiling for a campaign by joining
    /// `gm_campaign_accounts → gm_auto_reply_config`.
    ///
    /// **Scheme A (strictest)**: when a campaign binds multiple social
    /// accounts that each carry their own `auto_reply_config`, the limit
    /// is the **MIN** across all bound configs. This keeps the (campaign,
    /// user_ref) 24h ceiling deterministic regardless of which SA handled
    /// the latest reply — the most restrictive config wins. Returns None
    /// when no bound account has a config row so callers can apply a
    /// default.
    pub fn rate_limit_for_campaign(&self, campaign_id: i32) -> Result<Option<i32>, DieselError> {
        use diesel::dsl::min;
        let mut conn = self.pool.get().expect("DB connection error");
        gm_campaign_accounts::table
            .inner_join(
                gm_auto_reply_config::table
                    .on(gm_auto_reply_config::social_account_id
                        .eq(gm_campaign_accounts::account_id)),
            )
            .filter(gm_campaign_accounts::campaign_id.eq(campaign_id))
            .select(min(gm_auto_reply_config::rate_limit_per_day))
            .first::<Option<i32>>(&mut conn)
    }

    /// UPSERT a reply log row keyed by `inbound_msg_id` and apply the M3 §4
    /// state-transition contract: terminal statuses (sent/escalated/failed)
    /// are immutable; pending → terminal is allowed; identical status is a noop.
    pub fn upsert_reply_log(
        &self,
        req: &ReplyLogUpsertRequest,
    ) -> Result<ReplyLogUpsertResponse, DieselError> {
        let mut conn = self.pool.get().expect("DB connection error");
        let req = req.clone();
        conn.transaction(|tx| {
            let existing: Option<DmReplyLog> = gm_dm_reply_log::table
                .filter(gm_dm_reply_log::inbound_msg_id.eq(&req.inbound_msg_id))
                .select(DmReplyLog::as_select())
                .for_update()
                .first::<DmReplyLog>(tx)
                .optional()?;

            match existing {
                None => {
                    let row = NewDmReplyLog {
                        inbound_msg_id: req.inbound_msg_id,
                        conv_id: req.conv_id,
                        social_account_id: req.social_account_id,
                        campaign_id: req.campaign_id,
                        user_ref: req.user_ref,
                        user_handle: req.user_handle,
                        platform: req.platform,
                        inbound_text: req.inbound_text,
                        inbound_received_at: req.inbound_received_at,
                        status: req.status,
                        skip_reason: req.skip_reason,
                        rag_score: req.rag_score,
                        llm_confidence: req.llm_confidence,
                        llm_model: req.llm_model,
                        latency_ms: req.latency_ms,
                        reply_text: req.reply_text,
                        escalate_reason: req.escalate_reason,
                    };
                    diesel::insert_into(gm_dm_reply_log::table)
                        .values(&row)
                        .execute(tx)?;
                    Ok(ReplyLogUpsertResponse {
                        action: REPLY_ACTION_CREATED.to_string(),
                        existed_status: None,
                    })
                }
                Some(prev) => {
                    let prev_status = prev.status.clone();
                    if prev_status == req.status {
                        return Ok(ReplyLogUpsertResponse {
                            action: REPLY_ACTION_NOOP.to_string(),
                            existed_status: Some(prev_status),
                        });
                    }
                    if TERMINAL_STATUSES.contains(&prev_status.as_str()) {
                        return Ok(ReplyLogUpsertResponse {
                            action: REPLY_ACTION_NOOP.to_string(),
                            existed_status: Some(prev_status),
                        });
                    }
                    if prev_status == "pending" && TERMINAL_STATUSES.contains(&req.status.as_str())
                    {
                        diesel::update(
                            gm_dm_reply_log::table.filter(gm_dm_reply_log::id.eq(prev.id)),
                        )
                        .set((
                            gm_dm_reply_log::status.eq(&req.status),
                            gm_dm_reply_log::reply_text.eq(req.reply_text.clone()),
                            gm_dm_reply_log::escalate_reason.eq(req.escalate_reason.clone()),
                            gm_dm_reply_log::rag_score.eq(req.rag_score),
                            gm_dm_reply_log::llm_confidence.eq(req.llm_confidence),
                            gm_dm_reply_log::llm_model.eq(req.llm_model.clone()),
                            gm_dm_reply_log::latency_ms.eq(req.latency_ms),
                            gm_dm_reply_log::updated_at.eq(Utc::now()),
                        ))
                        .execute(tx)?;
                        return Ok(ReplyLogUpsertResponse {
                            action: REPLY_ACTION_UPDATED.to_string(),
                            existed_status: Some(prev_status),
                        });
                    }
                    Ok(ReplyLogUpsertResponse {
                        action: REPLY_ACTION_NOOP.to_string(),
                        existed_status: Some(prev_status),
                    })
                }
            }
        })
    }

    pub fn get_reply_log_by_inbound_id(
        &self,
        inbound_msg_id: &str,
    ) -> Result<Option<DmReplyLog>, DieselError> {
        let mut conn = self.pool.get().expect("DB connection error");
        gm_dm_reply_log::table
            .filter(gm_dm_reply_log::inbound_msg_id.eq(inbound_msg_id))
            .select(DmReplyLog::as_select())
            .first::<DmReplyLog>(&mut conn)
            .optional()
    }

    pub fn mark_conversation_review(
        &self,
        conv_id: &str,
        reason: &str,
    ) -> Result<DmConversationReview, DieselError> {
        let mut conn = self.pool.get().expect("DB connection error");
        let row = NewDmConversationReview {
            conv_id: conv_id.to_string(),
            needs_human_review: true,
            human_review_reason: Some(reason.to_string()),
        };
        diesel::insert_into(gm_dm_conversation_review::table)
            .values(&row)
            .on_conflict(gm_dm_conversation_review::conv_id)
            .do_update()
            .set((
                gm_dm_conversation_review::needs_human_review.eq(true),
                gm_dm_conversation_review::human_review_reason.eq(reason),
                gm_dm_conversation_review::resolved_at.eq::<Option<DateTime<Utc>>>(None),
                gm_dm_conversation_review::resolved_by.eq::<Option<i32>>(None),
                gm_dm_conversation_review::updated_at.eq(Utc::now()),
            ))
            .returning(DmConversationReview::as_returning())
            .get_result(&mut conn)
    }

    pub fn clear_conversation_review(
        &self,
        conv_id: &str,
        resolved_by: i32,
    ) -> Result<DmConversationReview, DieselError> {
        let mut conn = self.pool.get().expect("DB connection error");
        diesel::update(
            gm_dm_conversation_review::table.filter(gm_dm_conversation_review::conv_id.eq(conv_id)),
        )
        .set((
            gm_dm_conversation_review::needs_human_review.eq(false),
            gm_dm_conversation_review::resolved_at.eq(Some(Utc::now())),
            gm_dm_conversation_review::resolved_by.eq(Some(resolved_by)),
            gm_dm_conversation_review::updated_at.eq(Utc::now()),
        ))
        .returning(DmConversationReview::as_returning())
        .get_result(&mut conn)
    }

    pub fn get_conversation_review(
        &self,
        conv_id: &str,
    ) -> Result<Option<DmConversationReview>, DieselError> {
        let mut conn = self.pool.get().expect("DB connection error");
        gm_dm_conversation_review::table
            .filter(gm_dm_conversation_review::conv_id.eq(conv_id))
            .select(DmConversationReview::as_select())
            .first::<DmConversationReview>(&mut conn)
            .optional()
    }

    /// §4 contract: find the most recent escalated `dm_reply_log` row for
    /// this conversation and stamp `resolved_at` / `resolved_by`. Returns
    /// `Ok(None)` when no escalated row exists (conversation may have
    /// been resolved by direct manual intervention).
    pub fn resolve_latest_escalated_reply_log(
        &self,
        conv_id: &str,
        resolved_by: i32,
    ) -> Result<Option<i64>, DieselError> {
        let mut conn = self.pool.get().expect("DB connection error");
        let target_id: Option<i64> = gm_dm_reply_log::table
            .filter(gm_dm_reply_log::conv_id.eq(conv_id))
            .filter(gm_dm_reply_log::status.eq("escalated"))
            .order(gm_dm_reply_log::created_at.desc())
            .select(gm_dm_reply_log::id)
            .first::<i64>(&mut conn)
            .optional()?;
        let Some(id) = target_id else {
            return Ok(None);
        };
        let now = Utc::now();
        diesel::update(gm_dm_reply_log::table.filter(gm_dm_reply_log::id.eq(id)))
            .set((
                gm_dm_reply_log::resolved_at.eq(Some(now)),
                gm_dm_reply_log::resolved_by.eq(Some(resolved_by)),
                gm_dm_reply_log::updated_at.eq(now),
            ))
            .execute(&mut conn)?;
        Ok(Some(id))
    }

    /// Helper for tests: kept around in case the planned 30-day window query
    /// is reinstated after a `campaign_interactions` table is introduced.
    #[allow(dead_code)]
    fn _thirty_days_ago() -> DateTime<Utc> {
        Utc::now() - Duration::days(30)
    }
}
