use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PatrolReportsQuery {
    pub report_type: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PatrolTrendQuery {
    pub days: Option<i32>,
    pub report_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PatrolReportDto {
    pub id: i32,
    pub report_id: String,
    pub report_type: String,
    pub user_id: i32,
    pub device_id: String,
    pub started_at: String,
    pub completed_at: String,
    pub total_accounts: i32,
    pub success_count: i32,
    pub error_count: i32,
    pub created_at: String,
    pub accounts: Vec<PatrolAccountStatsDto>,
}

#[derive(Debug, Serialize)]
pub struct PatrolAccountStatsDto {
    pub id: i32,
    pub report_type: String,
    pub social_account_id: i32,
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
    pub collected_at: String,
}

#[derive(Debug, Serialize)]
pub struct PatrolReportsResponse {
    pub reports: Vec<PatrolReportDto>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Serialize)]
pub struct PatrolSummaryDto {
    pub total_reports: i64,
    pub total_accounts_monitored: i64,
    pub latest_profile_report: Option<PatrolReportBriefDto>,
    pub latest_notification_report: Option<PatrolReportBriefDto>,
}

#[derive(Debug, Serialize)]
pub struct PatrolReportBriefDto {
    pub report_id: String,
    pub report_type: String,
    pub total_accounts: i32,
    pub success_count: i32,
    pub error_count: i32,
    pub completed_at: String,
}

#[derive(Debug, Serialize)]
pub struct PatrolLatestDto {
    pub profile_report: Option<PatrolReportDto>,
    pub notification_report: Option<PatrolReportDto>,
    pub account_stats: Vec<PatrolAccountStatsDto>,
}
