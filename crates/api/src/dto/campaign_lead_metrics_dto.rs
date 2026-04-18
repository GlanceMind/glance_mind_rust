use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Campaign 级反馈数据聚合结果。
///
/// 语义说明：本 DTO 代表 "该 campaign 关联账号在
/// `[window_start_at, window_end_at]` 区间内收到的互动总量"，
/// **不**代表 "由 campaign 动作直接产生的互动"。
/// 同一社交账号若同时在多个 campaign 下，每个 campaign 都会看到它的互动。
///
/// 聚合来源：仅 `gm_patrol_account_stats.report_type = 'notification'`
/// 的增量行。`profile` 为快照类型，不参与求和。
///
/// `tracked_account_count` 由 repo 层兜底保证 `<= account_count`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CampaignLeadMetricsDto {
    pub campaign_id: i32,
    pub window_start_at: DateTime<Utc>,
    pub window_end_at: DateTime<Utc>,
    /// 本 campaign 下的账号总数（来自 gm_campaign_accounts，无论有无 patrol 数据）
    pub account_count: i64,
    /// 窗口内实际产出过至少 1 条 notification 的账号数
    pub tracked_account_count: i64,
    pub new_followers: i64,
    pub dms: i64,
    pub friend_requests: i64,
    pub mentions: i64,
    /// 窗口内累计收到的点赞数（来自 notification 行的 received_likes）
    pub received_likes: i64,
    /// 窗口内累计收到的评论数（来自 notification 行的 received_comments）
    pub received_comments: i64,
    /// 窗口内最后一次 notification 采集时间；无数据时为 None
    pub last_updated_at: Option<DateTime<Utc>>,
}
