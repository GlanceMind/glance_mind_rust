use crate::config::database::Database;
use crate::dto::dm_auto_reply_dto::{
    ClearReviewResponse, DmConfigResponse, EscalateResponse, InternalReplyResponse,
    RateLimitResponse, ReplyLogUpsertRequest, ReplyLogUpsertResponse,
};
use crate::error::api_error::ApiError;
use crate::error::db_error::DbError;
use crate::repository::dm_auto_reply_repository::DmAutoReplyRepository;
use crate::repository::social_account_repository::SocialAccountRepository;
use crate::service::nats_dm_service::NatsDmService;
use chrono::Utc;
use std::sync::Arc;

/// Matches the `DEFAULT 5` set in migration up.sql for
/// `gm_auto_reply_config.rate_limit_per_day`; used when a campaign has no
/// bound `auto_reply_config` row yet.
const DEFAULT_RATE_LIMIT_PER_DAY: i32 = 5;

#[derive(Clone)]
pub struct DmAutoReplyService {
    repo: DmAutoReplyRepository,
    social_account_repo: SocialAccountRepository,
    nats: Option<NatsDmService>,
}

impl DmAutoReplyService {
    pub fn new(db: &Arc<Database>, nats: Option<NatsDmService>) -> Self {
        Self {
            repo: DmAutoReplyRepository::new(db.pool.clone()),
            social_account_repo: SocialAccountRepository::new(db.pool.clone()),
            nats,
        }
    }

    pub fn repository(&self) -> &DmAutoReplyRepository {
        &self.repo
    }

    pub fn get_config(&self, social_account_id: i32) -> Result<DmConfigResponse, ApiError> {
        let cfg = self
            .repo
            .get_auto_reply_config(social_account_id)
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let campaign_id = self
            .repo
            .resolve_campaign_id(social_account_id, "")
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        // root §5.3: Gateway maps knowledge_id from campaign_id; keep the
        // contract `campaign_{id}` so Dify KB selection stays in one place
        // and doesn't depend on gm_product_faq topology.
        let knowledge_id = campaign_id.map(|c| format!("campaign_{}", c));

        let (product_info, dm_prompt) = match campaign_id {
            Some(cid) => {
                let ctx = self
                    .repo
                    .get_campaign_context(cid)
                    .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;
                match ctx {
                    Some(c) => (c.product_info, c.dm_prompt),
                    None => (None, None),
                }
            }
            None => (None, None),
        };

        // Per-(campaign, user) rate limit window is (campaign_id, user_ref);
        // the config endpoint does not know user_ref, so `rate_limit_count_today`
        // here is left at 0 and the Gateway should call `/internal/dm/rate-limit/
        // :campaign_id/:user_ref` for the authoritative counter.
        let cfg = match cfg {
            Some(c) => c,
            None => {
                return Ok(DmConfigResponse {
                    social_account_id,
                    enabled: false,
                    campaign_id,
                    dm_prompt,
                    product_info,
                    knowledge_id,
                    brand_name: String::new(),
                    blacklist_keywords: vec![],
                    rate_limit_per_day: 0,
                    rate_limit_count_today: 0,
                    confidence_threshold: 0.0,
                    rag_threshold: 0.0,
                    fallback_strategy: "escalate".to_string(),
                });
            }
        };

        Ok(DmConfigResponse {
            social_account_id,
            enabled: cfg.enabled,
            campaign_id,
            dm_prompt,
            product_info,
            knowledge_id,
            brand_name: cfg.brand_name,
            blacklist_keywords: cfg.blacklist_keywords.into_iter().flatten().collect(),
            rate_limit_per_day: cfg.rate_limit_per_day,
            rate_limit_count_today: 0,
            confidence_threshold: cfg.confidence_threshold,
            rag_threshold: cfg.rag_threshold,
            fallback_strategy: cfg.fallback_strategy,
        })
    }

    pub fn get_rate_limit(
        &self,
        campaign_id: i32,
        user_ref: &str,
    ) -> Result<RateLimitResponse, ApiError> {
        let count = self
            .repo
            .count_sent_replies_today(campaign_id, user_ref)
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;
        let limit = self
            .repo
            .rate_limit_for_campaign(campaign_id)
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?
            .unwrap_or(DEFAULT_RATE_LIMIT_PER_DAY);
        Ok(RateLimitResponse {
            campaign_id,
            user_ref: user_ref.to_string(),
            count: count.try_into().unwrap_or(i32::MAX),
            limit,
        })
    }

    pub fn upsert_reply_log(
        &self,
        req: ReplyLogUpsertRequest,
    ) -> Result<ReplyLogUpsertResponse, ApiError> {
        if !is_valid_status(&req.status) {
            return Err(ApiError::BadRequest(format!(
                "invalid status: {}",
                req.status
            )));
        }
        self.repo
            .upsert_reply_log(&req)
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))
    }

    /// Send a reply via NATS jetstream. Wraps `nats_dm_service::send_reply` so
    /// we keep audit/cmd_id semantics consistent with user-driven replies.
    pub async fn send_reply(
        &self,
        conv_id: &str,
        content: &str,
        inbound_msg_id: &str,
    ) -> Result<InternalReplyResponse, ApiError> {
        let nats = self
            .nats
            .as_ref()
            .ok_or_else(|| ApiError::InternalServerError("NATS DM service not available".into()))?;

        let (social_account_id, remote_username) = parse_conv_id(conv_id)?;
        if social_account_id <= 0 {
            return Err(ApiError::BadRequest(
                "internal reply requires a real social account id".into(),
            ));
        }

        let account = self
            .social_account_repo
            .find_by_id(social_account_id)
            .await
            .map_err(|e| match e {
                diesel::result::Error::NotFound => {
                    ApiError::NotFound(format!("social_account {} not found", social_account_id))
                }
                other => ApiError::from(DbError::SomethingWentWrong(other.to_string())),
            })?;
        let device_id = account.device_id.clone().unwrap_or_default();
        if device_id.trim().is_empty() {
            return Err(ApiError::BadRequest(
                "social account missing device_id".into(),
            ));
        }
        let profile_name = account.profile_name.clone().unwrap_or_default();
        let resp = nats
            .send_reply(
                conv_id,
                &device_id,
                account.id,
                account.platform_id,
                &profile_name,
                remote_username,
                content,
                "text",
            )
            .await?;

        let _ = inbound_msg_id;

        Ok(InternalReplyResponse {
            cmd_id: resp.cmd_id,
        })
    }

    pub fn escalate(
        &self,
        conv_id: &str,
        reason: &str,
        _inbound_msg_id: &str,
    ) -> Result<EscalateResponse, ApiError> {
        let row = self
            .repo
            .mark_conversation_review(conv_id, reason)
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;
        // TODO(M3->post-MVP): notify via notification_service so reviewers
        // get a real-time alert. For MVP we emit a tracing event so M5 can
        // pick it up from logs.
        tracing::info!(
            target = "dm_auto_reply",
            event = "dm_auto_reply.escalated",
            conv_id = %row.conv_id,
            reason = %reason,
            "DM conversation escalated for human review"
        );
        Ok(EscalateResponse {
            conv_id: row.conv_id,
            needs_human_review: row.needs_human_review,
        })
    }

    pub fn clear_review(
        &self,
        conv_id: &str,
        resolved_by: i32,
    ) -> Result<ClearReviewResponse, ApiError> {
        let resolved_reply_log_id = self
            .repo
            .resolve_latest_escalated_reply_log(conv_id, resolved_by)
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let existing = self
            .repo
            .get_conversation_review(conv_id)
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;
        let row = match existing {
            Some(_) => self
                .repo
                .clear_conversation_review(conv_id, resolved_by)
                .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?,
            None => {
                // Idempotent clear: no review row means there's nothing to
                // unset, so synthesise a "now" response without inserting a
                // synthetic escalation.
                return Ok(ClearReviewResponse {
                    conv_id: conv_id.to_string(),
                    resolved_at: Utc::now(),
                    resolved_reply_log_id,
                });
            }
        };

        Ok(ClearReviewResponse {
            conv_id: row.conv_id,
            resolved_at: row.resolved_at.unwrap_or_else(Utc::now),
            resolved_reply_log_id,
        })
    }
}

fn is_valid_status(s: &str) -> bool {
    matches!(s, "skipped" | "pending" | "sent" | "escalated" | "failed")
}

fn parse_conv_id(conv_id: &str) -> Result<(i32, &str), ApiError> {
    let mut parts = conv_id.splitn(2, '_');
    let id_str = parts
        .next()
        .ok_or_else(|| ApiError::BadRequest("invalid conv_id".into()))?;
    let remote = parts
        .next()
        .ok_or_else(|| ApiError::BadRequest("invalid conv_id".into()))?;
    let id = id_str
        .parse::<i32>()
        .map_err(|_| ApiError::BadRequest("invalid conv_id account id".into()))?;
    Ok((id, remote))
}
