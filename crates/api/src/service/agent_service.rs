use crate::dto::agent_dto::{
    CommentWithVideoDto, DeviceCommentsQuery, DeviceCommentsResponse, UnifiedCommentDto,
    UnifiedCommentWithConfigDto, UpdateCommentStatusDto, UpdateStatusResponse,
};
use crate::dto::common::{PageRequest, PageResponse};
use crate::error::api_error::ApiError;
use crate::platform_routing::SupportedPlatform;
use crate::repository::agent_repository::AgentRepository;
use crate::service::redis_service::RedisService;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error as DieselError;
use diesel::PgConnection;
use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet};

type AccountQuotaEntry = (i32, Option<String>, i32);

/// Per-group assembly context: platform-matched accounts + total account count.
/// The total count is used to distinguish:
///   - truly empty group (total == 0)  → keep comment (legacy passthrough)
///   - accounts exist but none match platform (total > 0, matched == 0) → drop comment + warn
struct GroupAssemblyCtx {
    /// Platform-matched ACTIVE accounts (id, profile_name, daily_max_replies)
    matched_accounts: Vec<AccountQuotaEntry>,
    /// Total ACTIVE accounts in the group (regardless of platform)
    total_account_count: i64,
}

#[derive(Clone)]
pub struct AgentService {
    agent_repo: AgentRepository,
    redis_service: Option<RedisService>,
}

impl AgentService {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            agent_repo: AgentRepository::new(pool),
            redis_service: None,
        }
    }

    pub fn set_redis_service(&mut self, svc: RedisService) {
        self.redis_service = Some(svc);
    }

    /// Legacy method - only queries TikTok comments
    pub async fn get_video_comments(
        &self,
        video_id: i32,
        req: PageRequest,
    ) -> Result<PageResponse<crate::dto::agent_dto::AgentCommentDto>, ApiError> {
        self.agent_repo
            .get_video_comments(video_id, req.page, req.page_size)
            .map_err(|e| ApiError::InternalServerError(e.to_string()))
    }

    /// Unified method - queries comments for any platform
    pub async fn get_unified_comments(
        &self,
        content_db_id: i32,
        platform_id: i32,
        req: PageRequest,
    ) -> Result<PageResponse<UnifiedCommentDto>, ApiError> {
        self.agent_repo
            .get_unified_comments(content_db_id, platform_id, req.page, req.page_size)
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::BadRequest(format!("Invalid platform_id: {}", platform_id))
                }
                _ => ApiError::InternalServerError(e.to_string()),
            })
    }

    // New method for device-based query with platform support
    // Returns protocol-compliant structure: campaign config + comments array
    pub async fn get_comments_by_device(
        &self,
        query: DeviceCommentsQuery,
    ) -> Result<DeviceCommentsResponse, ApiError> {
        let page_i64 = i64::from(query.page);
        let per_page_i64 = i64::from(query.per_page);
        let platform = Self::parse_supported_platform_name(&query.platform)?;

        let page_response = self
            .agent_repo
            .get_comments_by_device_unified(
                &query.device_id,
                platform,
                query.status,
                page_i64,
                per_page_i64,
            )
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        let filtered = self.enforce_daily_limits(page_response.list);
        let filtered_total = filtered.len() as i64;

        UnifiedCommentWithConfigDto::to_protocol_response(
            filtered,
            filtered_total,
            query.page,
            query.per_page,
        )
    }

    /// Service-layer post-processing: dedup by comment_id, then quota-aware profile reassignment.
    ///
    /// Fetches campaign→(group, platform) and group→accounts mappings from DB, then delegates
    /// to `enforce_daily_limits_inner` for pure-logic processing.
    pub fn enforce_daily_limits(
        &self,
        comments: Vec<UnifiedCommentWithConfigDto>,
    ) -> Vec<UnifiedCommentWithConfigDto> {
        if comments.is_empty() {
            return comments;
        }

        // Dedup first (always applied)
        let deduped = Self::dedup_comments(comments);

        // Collect distinct campaign_ids
        let campaign_ids: Vec<i32> = deduped
            .iter()
            .filter_map(|c| c.campaign_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        // Batch-query campaign → (group_id, platform_id) from DB
        // Triple: (campaign_id, group_id, platform_id)
        let campaign_triples: Vec<(i32, i32, i32)> = self
            .agent_repo
            .get_campaign_group_ids(&campaign_ids)
            .unwrap_or_default();

        // campaign_id → (group_id, platform_id)
        let campaign_to_group_platform: HashMap<i32, (i32, i32)> = campaign_triples
            .iter()
            .map(|&(cid, gid, pid)| (cid, (gid, pid)))
            .collect();

        // Cache group+platform → GroupAssemblyCtx from DB
        // Key: (group_id, platform_id)
        let mut group_ctx_map: HashMap<(i32, i32), GroupAssemblyCtx> = HashMap::new();
        for &(_cid, gid, pid) in &campaign_triples {
            let key = (gid, pid);
            if group_ctx_map.contains_key(&key) {
                continue;
            }
            let matched_accounts = self
                .agent_repo
                .get_group_active_accounts(gid, pid)
                .unwrap_or_default();
            let total_account_count = self.agent_repo.get_group_account_count(gid).unwrap_or(0);
            group_ctx_map.insert(
                key,
                GroupAssemblyCtx {
                    matched_accounts,
                    total_account_count,
                },
            );
        }

        Self::enforce_daily_limits_inner(
            deduped,
            &campaign_to_group_platform,
            &group_ctx_map,
            self.redis_service.as_ref(),
        )
    }

    /// Dedup comments by `comment_id`, keeping first occurrence and preserving order.
    fn dedup_comments(
        comments: Vec<UnifiedCommentWithConfigDto>,
    ) -> Vec<UnifiedCommentWithConfigDto> {
        let mut seen = HashSet::new();
        comments
            .into_iter()
            .filter(|c| seen.insert(c.comment_id.clone()))
            .collect()
    }

    /// Pure-logic quota enforcement. Testable without DB.
    ///
    /// - `campaign_to_group_platform`: campaign_id → (social_group_id, platform_id)
    /// - `group_ctx_map`: (group_id, platform_id) → GroupAssemblyCtx
    ///   (matched_accounts = platform-filtered ACTIVE accounts;
    ///   total_account_count = all ACTIVE accounts in the group)
    /// - `redis`: None means skip quota checks (keep all comments)
    ///
    /// Empty-group semantics (preserved from pre-M4):
    ///   - group truly empty (total_account_count == 0) → keep comment (legacy passthrough)
    ///   - group has accounts but none match campaign platform → DROP comment + warn
    fn enforce_daily_limits_inner(
        comments: Vec<UnifiedCommentWithConfigDto>,
        campaign_to_group_platform: &HashMap<i32, (i32, i32)>,
        group_ctx_map: &HashMap<(i32, i32), GroupAssemblyCtx>,
        redis: Option<&RedisService>,
    ) -> Vec<UnifiedCommentWithConfigDto> {
        let redis = match redis {
            Some(r) => r,
            None => return comments,
        };

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let mut result = Vec::with_capacity(comments.len());
        for mut comment in comments {
            let group_platform = comment
                .campaign_id
                .and_then(|cid| campaign_to_group_platform.get(&cid).copied());

            let (group_id, platform_id) = match group_platform {
                Some(gp) => gp,
                None => {
                    // No group assigned to campaign — keep comment (passthrough)
                    result.push(comment);
                    continue;
                }
            };

            let ctx = group_ctx_map.get(&(group_id, platform_id));

            match ctx {
                Some(c) if !c.matched_accounts.is_empty() => {
                    // Normal path: platform-matched accounts exist — do quota assignment
                    let mut shuffled = c.matched_accounts.clone();
                    shuffled.shuffle(&mut rand::rng());

                    let mut assigned = false;
                    let cmt_id_for_log = comment.comment_id.clone();
                    for (account_id, profile_name, daily_limit) in &shuffled {
                        match redis.try_reserve(*account_id, &today, *daily_limit) {
                            Ok(true) => {
                                comment.profile_name = profile_name.clone();
                                result.push(comment);
                                assigned = true;
                                break;
                            }
                            Ok(false) => continue,
                            Err(e) => {
                                tracing::warn!("Redis reserve error for account {account_id}: {e}");
                                result.push(comment);
                                assigned = true;
                                break;
                            }
                        }
                    }

                    if !assigned {
                        tracing::debug!(
                            "All accounts in group {} exhausted daily quota, dropping comment {}",
                            group_id,
                            cmt_id_for_log
                        );
                    }
                }
                Some(c) if c.total_account_count > 0 => {
                    // Group has ACTIVE accounts but none match the campaign platform.
                    // This is a misconfiguration: EXCLUDE the comment and warn.
                    tracing::warn!(
                        campaign_id = comment.campaign_id,
                        group_id = group_id,
                        platform_id = platform_id,
                        skipped_account_count = c.total_account_count,
                        "Dropping comment: group has {} ACTIVE account(s) but none match \
                         campaign platform_id={}; check account platform assignments",
                        c.total_account_count,
                        platform_id
                    );
                    // comment dropped — not pushed to result
                }
                _ => {
                    // Truly empty group (0 ACTIVE accounts total) or ctx missing:
                    // keep comment — legacy passthrough semantics preserved.
                    result.push(comment);
                    continue;
                }
            }
        }

        result
    }

    // Legacy method for backward compatibility (TikTok only)
    #[allow(dead_code)]
    pub async fn get_tiktok_comments_by_device(
        &self,
        query: DeviceCommentsQuery,
    ) -> Result<PageResponse<CommentWithVideoDto>, ApiError> {
        // Convert i32 to i64 for repository layer
        let page_i64 = i64::from(query.page);
        let per_page_i64 = i64::from(query.per_page);

        self.agent_repo
            .get_comments_by_device(&query.device_id, query.status, page_i64, per_page_i64)
            .map_err(|e| ApiError::InternalServerError(e.to_string()))
    }

    pub async fn update_comment_status(
        &self,
        dto: UpdateCommentStatusDto,
    ) -> Result<UpdateStatusResponse, ApiError> {
        let platform = match dto.platform.as_deref() {
            Some(platform) => Self::parse_supported_platform_name(platform)?,
            None => SupportedPlatform::Tiktok,
        };

        let affected = self
            .agent_repo
            .update_comment_status(&dto.comment_id, dto.status, platform)
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        if affected == 0 {
            return Err(ApiError::NotFound(format!(
                "Comment {} not found",
                dto.comment_id
            )));
        }

        Ok(UpdateStatusResponse {
            success: true,
            message: "Status updated successfully".to_string(),
        })
    }

    pub async fn get_all_campaign_comments_unified(
        &self,
        campaign_id: i32,
        platform_id: i32,
    ) -> Result<Vec<UnifiedCommentDto>, ApiError> {
        self.agent_repo
            .get_all_unified_comments_by_campaign(campaign_id, platform_id)
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::BadRequest(format!("Invalid platform_id: {}", platform_id))
                }
                _ => ApiError::InternalServerError(e.to_string()),
            })
    }

    fn parse_supported_platform_name(platform: &str) -> Result<SupportedPlatform, ApiError> {
        SupportedPlatform::from_name(platform)
            .ok_or_else(|| ApiError::BadRequest(format!("Unsupported platform: {}", platform)))
    }

    /// Get all videos for a campaign (for export)
    pub async fn get_campaign_videos(
        &self,
        campaign_id: i32,
    ) -> Result<Vec<glance_mind_db::entity::agent::AgentVideo>, ApiError> {
        self.agent_repo
            .get_campaign_videos(campaign_id)
            .map_err(|e| ApiError::InternalServerError(e.to_string()))
    }

    /// Get all comments for a campaign (for export)
    pub async fn get_campaign_comments(
        &self,
        campaign_id: i32,
    ) -> Result<Vec<glance_mind_db::entity::agent::AgentComment>, ApiError> {
        self.agent_repo
            .get_campaign_comments(campaign_id)
            .map_err(|e| ApiError::InternalServerError(e.to_string()))
    }
}

// ============================================================================
// Unit Tests — enforce_daily_limits logic
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn mock_comment(
        id: i32,
        comment_id: &str,
        campaign_id: Option<i32>,
    ) -> UnifiedCommentWithConfigDto {
        UnifiedCommentWithConfigDto {
            id,
            comment_id: comment_id.to_string(),
            content_id: format!("content_{}", id),
            platform: "tiktok".to_string(),
            content: Some(format!("Comment text {}", id)),
            status: "pending".to_string(),
            user_nickname: Some(format!("user_{}", id)),
            user_unique_id: Some(format!("uid_{}", id)),
            suggested_reply: Some("reply".to_string()),
            suggested_dm: None,
            suggested_reply_post: None,
            reason: Some("test".to_string()),
            create_time: chrono::DateTime::from_timestamp(1705000000, 0).map(|dt| dt.naive_utc()),
            created_at: Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap(),
            campaign_id,
            auto_like: true,
            auto_follow: false,
            auto_dm: false,
            auto_reply_comments: true,
            auto_reply_post: false,
            profile_name: Some("OriginalProfile".to_string()),
            content_url: None,
            content_type: Some("VIDEO".to_string()),
            author_unique_id: None,
            comment_url: None,
            comment_user_url: None,
        }
    }

    fn get_test_redis() -> Option<RedisService> {
        let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
        let client = redis::Client::open(url.as_str()).ok()?;
        let mut conn = client.get_connection().ok()?;
        let _: Result<String, _> = redis::cmd("PING").query(&mut conn);
        Some(RedisService::new(client))
    }

    fn flush_keys(svc: &RedisService, pattern_account_ids: &[i32]) {
        if let Ok(mut conn) = svc.client.get_connection() {
            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
            for aid in pattern_account_ids {
                let key = format!("reply_quota:{}:{}", aid, today);
                let _: Result<(), _> = redis::cmd("DEL").arg(&key).query(&mut conn);
            }
        }
    }

    // ========================================================================
    // 1. test_dedup_by_comment_id
    // ========================================================================
    #[test]
    fn test_dedup_by_comment_id() {
        let comments = vec![
            mock_comment(1, "cmt_aaa", Some(100)),
            mock_comment(2, "cmt_aaa", Some(100)), // duplicate
            mock_comment(3, "cmt_bbb", Some(100)),
            mock_comment(4, "cmt_bbb", Some(100)), // duplicate
            mock_comment(5, "cmt_ccc", Some(100)),
        ];

        let result = AgentService::dedup_comments(comments);

        let ids: Vec<&str> = result.iter().map(|c| c.comment_id.as_str()).collect();
        assert_eq!(ids, vec!["cmt_aaa", "cmt_bbb", "cmt_ccc"]);
        assert_eq!(result.len(), 3);
    }

    // ========================================================================
    // 2. test_dedup_preserves_order
    // ========================================================================
    #[test]
    fn test_dedup_preserves_order() {
        let comments = vec![
            mock_comment(1, "first", Some(1)),
            mock_comment(2, "second", Some(1)),
            mock_comment(3, "first", Some(1)), // dup of "first"
            mock_comment(4, "third", Some(1)),
            mock_comment(5, "second", Some(1)), // dup of "second"
        ];

        let result = AgentService::dedup_comments(comments);

        let ids: Vec<&str> = result.iter().map(|c| c.comment_id.as_str()).collect();
        assert_eq!(ids, vec!["first", "second", "third"]);
        // Verify we kept the first occurrence (by DB id)
        assert_eq!(result[0].id, 1);
        assert_eq!(result[1].id, 2);
        assert_eq!(result[2].id, 4);
    }

    // Helper: build a GroupAssemblyCtx with matched accounts == total accounts
    // (i.e. no platform mismatch, standard case for most tests)
    fn make_ctx(accounts: Vec<AccountQuotaEntry>) -> GroupAssemblyCtx {
        let total = accounts.len() as i64;
        GroupAssemblyCtx {
            matched_accounts: accounts,
            total_account_count: total,
        }
    }

    // Helper: build a truly empty GroupAssemblyCtx (legacy passthrough)
    fn make_empty_ctx() -> GroupAssemblyCtx {
        GroupAssemblyCtx {
            matched_accounts: vec![],
            total_account_count: 0,
        }
    }

    // ========================================================================
    // 3. test_profile_reassignment
    // ========================================================================
    #[test]
    fn test_profile_reassignment() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => {
                eprintln!("Skipping test_profile_reassignment: Redis not available");
                return;
            }
        };

        // campaign 200 → group 8001, platform 3 (facebook)
        // Group 8001 has 3 platform-matched accounts
        let campaign_to_group_platform: HashMap<i32, (i32, i32)> = [(200, (8001, 3))].into();
        let group_ctx_map: HashMap<(i32, i32), GroupAssemblyCtx> = [(
            (8001, 3),
            make_ctx(vec![
                (7001, Some("Profile A".to_string()), 50),
                (7002, Some("Profile B".to_string()), 50),
                (7003, Some("Profile C".to_string()), 50),
            ]),
        )]
        .into();

        flush_keys(&svc, &[7001, 7002, 7003]);

        let comments = vec![
            mock_comment(1, "c1", Some(200)),
            mock_comment(2, "c2", Some(200)),
            mock_comment(3, "c3", Some(200)),
        ];

        let result = AgentService::enforce_daily_limits_inner(
            comments,
            &campaign_to_group_platform,
            &group_ctx_map,
            Some(&svc),
        );

        assert_eq!(result.len(), 3);
        let valid_profiles = ["Profile A", "Profile B", "Profile C"];
        for c in &result {
            let pn = c.profile_name.as_deref().unwrap();
            assert!(
                valid_profiles.contains(&pn),
                "profile_name '{}' not in group pool",
                pn
            );
        }

        flush_keys(&svc, &[7001, 7002, 7003]);
    }

    // ========================================================================
    // 4. test_quota_exhaustion_filters
    // ========================================================================
    #[test]
    fn test_quota_exhaustion_filters() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => {
                eprintln!("Skipping test_quota_exhaustion_filters: Redis not available");
                return;
            }
        };

        // campaign 300 → group 8002, platform 3; 1 account with limit=1
        let campaign_to_group_platform: HashMap<i32, (i32, i32)> = [(300, (8002, 3))].into();
        let group_ctx_map: HashMap<(i32, i32), GroupAssemblyCtx> = [(
            (8002, 3),
            make_ctx(vec![(7010, Some("Solo Profile".to_string()), 1)]),
        )]
        .into();

        flush_keys(&svc, &[7010]);

        let comments = vec![
            mock_comment(1, "q1", Some(300)),
            mock_comment(2, "q2", Some(300)),
            mock_comment(3, "q3", Some(300)),
        ];

        let result = AgentService::enforce_daily_limits_inner(
            comments,
            &campaign_to_group_platform,
            &group_ctx_map,
            Some(&svc),
        );

        assert_eq!(result.len(), 1, "Only 1 comment should survive (limit=1)");
        assert_eq!(result[0].comment_id, "q1");
        assert_eq!(result[0].profile_name.as_deref(), Some("Solo Profile"));

        flush_keys(&svc, &[7010]);
    }

    // ========================================================================
    // 5. test_multi_group_isolation
    // ========================================================================
    #[test]
    fn test_multi_group_isolation() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => {
                eprintln!("Skipping test_multi_group_isolation: Redis not available");
                return;
            }
        };

        // Two campaigns → two separate groups, each with 1 account (limit=1)
        let campaign_to_group_platform: HashMap<i32, (i32, i32)> =
            [(400, (8003, 3)), (401, (8004, 3))].into();
        let group_ctx_map: HashMap<(i32, i32), GroupAssemblyCtx> = [
            (
                (8003, 3),
                make_ctx(vec![(7020, Some("Group A Profile".to_string()), 1)]),
            ),
            (
                (8004, 3),
                make_ctx(vec![(7021, Some("Group B Profile".to_string()), 1)]),
            ),
        ]
        .into();

        flush_keys(&svc, &[7020, 7021]);

        let comments = vec![
            mock_comment(1, "iso_1", Some(400)),
            mock_comment(2, "iso_2", Some(400)), // should be dropped (group A exhausted)
            mock_comment(3, "iso_3", Some(401)),
            mock_comment(4, "iso_4", Some(401)), // should be dropped (group B exhausted)
        ];

        let result = AgentService::enforce_daily_limits_inner(
            comments,
            &campaign_to_group_platform,
            &group_ctx_map,
            Some(&svc),
        );

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].comment_id, "iso_1");
        assert_eq!(result[0].profile_name.as_deref(), Some("Group A Profile"));
        assert_eq!(result[1].comment_id, "iso_3");
        assert_eq!(result[1].profile_name.as_deref(), Some("Group B Profile"));

        flush_keys(&svc, &[7020, 7021]);
    }

    // ========================================================================
    // 6. test_redis_unavailable_fallback
    // ========================================================================
    #[test]
    fn test_redis_unavailable_fallback() {
        // No Redis → dedup applied, but all comments kept (no quota filtering)
        let campaign_to_group_platform: HashMap<i32, (i32, i32)> = [(500, (8005, 3))].into();
        let group_ctx_map: HashMap<(i32, i32), GroupAssemblyCtx> = [(
            (8005, 3),
            make_ctx(vec![(7030, Some("Fallback Profile".to_string()), 1)]),
        )]
        .into();

        let comments = vec![
            mock_comment(1, "fb_1", Some(500)),
            mock_comment(2, "fb_1", Some(500)), // duplicate
            mock_comment(3, "fb_2", Some(500)),
            mock_comment(4, "fb_3", Some(500)),
        ];

        // Dedup first (as enforce_daily_limits does)
        let deduped = AgentService::dedup_comments(comments);
        assert_eq!(deduped.len(), 3, "Dedup should reduce 4→3");

        // Inner with redis=None
        let result = AgentService::enforce_daily_limits_inner(
            deduped,
            &campaign_to_group_platform,
            &group_ctx_map,
            None, // Redis unavailable
        );

        assert_eq!(
            result.len(),
            3,
            "Without Redis, all deduped comments should be kept"
        );
        // profile_name should NOT be reassigned (stays original)
        assert_eq!(result[0].profile_name.as_deref(), Some("OriginalProfile"));
    }

    // ========================================================================
    // 7. test_empty_input
    // ========================================================================
    #[test]
    fn test_empty_input() {
        let result = AgentService::dedup_comments(vec![]);
        assert!(result.is_empty());

        let result = AgentService::enforce_daily_limits_inner(
            vec![],
            &HashMap::new(),
            &HashMap::new(),
            None,
        );
        assert!(result.is_empty());
    }

    // ========================================================================
    // 8. test_daily_limit_zero_drops_all
    // ========================================================================
    #[test]
    fn test_daily_limit_zero_drops_all() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => {
                eprintln!("Skipping test_daily_limit_zero_drops_all: Redis not available");
                return;
            }
        };

        let campaign_to_group_platform: HashMap<i32, (i32, i32)> = [(700, (8010, 3))].into();
        let group_ctx_map: HashMap<(i32, i32), GroupAssemblyCtx> = [(
            (8010, 3),
            make_ctx(vec![(7050, Some("Zero Limit Account".to_string()), 0)]),
        )]
        .into();

        let comments = vec![
            mock_comment(1, "z1", Some(700)),
            mock_comment(2, "z2", Some(700)),
        ];

        let result = AgentService::enforce_daily_limits_inner(
            comments,
            &campaign_to_group_platform,
            &group_ctx_map,
            Some(&svc),
        );

        assert_eq!(
            result.len(),
            0,
            "Zero-limit accounts should reject all comments"
        );
    }

    // ========================================================================
    // 9. test_account_shuffle_distribution
    // ========================================================================
    #[test]
    fn test_account_shuffle_distribution() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => {
                eprintln!("Skipping test_account_shuffle_distribution: Redis not available");
                return;
            }
        };

        let account_ids = [7060, 7061, 7062];
        let campaign_to_group_platform: HashMap<i32, (i32, i32)> = [(800, (8020, 3))].into();
        let group_ctx_map: HashMap<(i32, i32), GroupAssemblyCtx> = [(
            (8020, 3),
            make_ctx(vec![
                (7060, Some("A".to_string()), 200),
                (7061, Some("B".to_string()), 200),
                (7062, Some("C".to_string()), 200),
            ]),
        )]
        .into();

        flush_keys(&svc, &account_ids);

        let mut assignment_count: HashMap<String, usize> = HashMap::new();
        for i in 0..90 {
            let comments = vec![mock_comment(i, &format!("shuffle_{}", i), Some(800))];
            let result = AgentService::enforce_daily_limits_inner(
                comments,
                &campaign_to_group_platform,
                &group_ctx_map,
                Some(&svc),
            );
            if let Some(c) = result.first() {
                *assignment_count
                    .entry(c.profile_name.clone().unwrap_or_default())
                    .or_insert(0) += 1;
            }
        }

        // Each account should get roughly 30 out of 90 (±20 for randomness)
        for profile in ["A", "B", "C"] {
            let count = assignment_count.get(profile).copied().unwrap_or(0);
            assert!(
                (10..=60).contains(&count),
                "Profile '{}' got {} assignments (expected ~30 ± margin)",
                profile,
                count
            );
        }

        flush_keys(&svc, &account_ids);
    }

    // ========================================================================
    // 10. test_campaign_without_group
    // ========================================================================
    #[test]
    fn test_campaign_without_group() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => {
                eprintln!("Skipping test_campaign_without_group: Redis not available");
                return;
            }
        };

        // campaign 600 has no group mapping
        let campaign_to_group_platform: HashMap<i32, (i32, i32)> = HashMap::new();
        let group_ctx_map: HashMap<(i32, i32), GroupAssemblyCtx> = HashMap::new();

        let comments = vec![
            mock_comment(1, "ng_1", Some(600)),
            mock_comment(2, "ng_2", Some(600)),
            mock_comment(3, "ng_3", None), // no campaign_id at all
        ];

        let result = AgentService::enforce_daily_limits_inner(
            comments,
            &campaign_to_group_platform,
            &group_ctx_map,
            Some(&svc),
        );

        assert_eq!(
            result.len(),
            3,
            "All comments should be kept (no group → no filtering)"
        );
        // profile_name unchanged
        for c in &result {
            assert_eq!(c.profile_name.as_deref(), Some("OriginalProfile"));
        }
    }

    // ========================================================================
    // 11. test_platform_mismatch_drops_comment (M4 new behavior)
    // ========================================================================
    #[test]
    fn test_platform_mismatch_drops_comment() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => {
                eprintln!("Skipping test_platform_mismatch_drops_comment: Redis not available");
                return;
            }
        };

        // campaign 900 → group 8030, platform 3 (facebook).
        // Group has 1 ACTIVE account but it is on platform 1 (reddit) — zero platform-matched.
        // total_account_count=1 means the "non-empty mismatch" branch fires → DROP comment.
        let campaign_to_group_platform: HashMap<i32, (i32, i32)> = [(900, (8030, 3))].into();
        let group_ctx_map: HashMap<(i32, i32), GroupAssemblyCtx> = [(
            (8030, 3),
            GroupAssemblyCtx {
                matched_accounts: vec![], // zero platform-matched accounts
                total_account_count: 1,   // group has 1 account total (wrong platform)
            },
        )]
        .into();

        let comments = vec![
            mock_comment(1, "pm_1", Some(900)),
            mock_comment(2, "pm_2", Some(900)),
        ];

        let result = AgentService::enforce_daily_limits_inner(
            comments,
            &campaign_to_group_platform,
            &group_ctx_map,
            Some(&svc),
        );

        assert_eq!(
            result.len(),
            0,
            "Comments must be dropped when group has accounts but none match campaign platform (M4)"
        );
    }

    // ========================================================================
    // 12. test_truly_empty_group_keeps_comment (M4 passthrough preserved)
    // ========================================================================
    #[test]
    fn test_truly_empty_group_keeps_comment() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => {
                eprintln!("Skipping test_truly_empty_group_keeps_comment: Redis not available");
                return;
            }
        };

        // campaign 901 → group 8031, platform 3.
        // Group is truly empty (0 ACTIVE accounts total) → legacy passthrough → keep comment.
        let campaign_to_group_platform: HashMap<i32, (i32, i32)> = [(901, (8031, 3))].into();
        let group_ctx_map: HashMap<(i32, i32), GroupAssemblyCtx> = [(
            (8031, 3),
            GroupAssemblyCtx {
                matched_accounts: vec![],
                total_account_count: 0, // truly empty
            },
        )]
        .into();

        let comments = vec![
            mock_comment(1, "te_1", Some(901)),
            mock_comment(2, "te_2", Some(901)),
        ];

        let result = AgentService::enforce_daily_limits_inner(
            comments,
            &campaign_to_group_platform,
            &group_ctx_map,
            Some(&svc),
        );

        assert_eq!(
            result.len(),
            2,
            "Comments must be kept for truly empty groups (legacy passthrough)"
        );
    }
}
