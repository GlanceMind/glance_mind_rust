use crate::schema::{gm_patrol_account_stats, gm_patrol_reports};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = gm_patrol_reports)]
pub struct PatrolReportEntity {
    pub id: i32,
    pub report_id: String,
    pub report_type: String,
    pub user_id: i32,
    pub device_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub total_accounts: i32,
    pub success_count: i32,
    pub error_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = gm_patrol_reports)]
pub struct NewPatrolReport {
    pub report_id: String,
    pub report_type: String,
    pub user_id: i32,
    pub device_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub total_accounts: i32,
    pub success_count: i32,
    pub error_count: i32,
}

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = gm_patrol_account_stats)]
pub struct PatrolAccountStatsEntity {
    pub id: i32,
    pub report_id: String,
    pub report_type: String,
    pub social_account_id: i32,
    pub device_id: String,
    pub user_id: i32,
    pub platform_id: i32,
    pub platform_name: String,
    pub username: String,
    pub followers_count: i32,
    pub following_count: i32,
    pub posts_count: i32,
    pub total_likes: i32,
    pub new_followers: i32,
    pub received_likes: i32,
    pub received_comments: i32,
    pub received_dms: i32,
    pub received_shares: i32,
    pub received_mentions: i32,
    pub received_friend_requests: i32,
    pub unread_total: i32,
    pub partial: bool,
    pub error: Option<String>,
    pub collected_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
