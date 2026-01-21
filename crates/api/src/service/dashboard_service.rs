use crate::config::database::Database;
use crate::dto::wallet_dto::{OverviewStatsDto, PerformanceDataDto, RecentCampaignDto};
use crate::error::api_error::ApiError;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::dsl::sum;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::{BigInt, Int4, Nullable, Numeric, Text, Timestamptz};
use glance_mind_db::schema::{gm_agent_comments, gm_campaigns};
use std::sync::Arc;

#[derive(QueryableByName)]
struct CountResult {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct RecentCampaignRow {
    #[diesel(sql_type = Int4)]
    id: i32,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    status: String,
    #[diesel(sql_type = Text)]
    platform_name: String,
    #[diesel(sql_type = Numeric)]
    actual_consumption: BigDecimal,
    #[diesel(sql_type = Int4)]
    total_scanned: i32,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct DashboardService {
    _db: Arc<Database>,
}

impl DashboardService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self { _db: db.clone() }
    }

    pub async fn get_overview_stats(&self, user_id: i32) -> Result<OverviewStatsDto, ApiError> {
        let mut conn = self._db.pool.get().map_err(|e| {
            tracing::error!("Failed to get DB connection: {}", e);
            ApiError::InternalServerError("Database error".to_string())
        })?;

        // 1. Total Spent: Sum of actual_consumption for all user campaigns
        let total_spent: Option<BigDecimal> = gm_campaigns::table
            .filter(gm_campaigns::user_id.eq(user_id))
            .select(sum(gm_campaigns::actual_consumption))
            .first(&mut conn)
            .map_err(|e| {
                tracing::error!("Failed to fetch total spent: {}", e);
                ApiError::InternalServerError("Database error".to_string())
            })?;

        // 2. Active Campaigns: Count of campaigns with status 'Active' or 'ACTIVE'
        let active_campaigns: i64 = gm_campaigns::table
            .filter(gm_campaigns::user_id.eq(user_id))
            .filter(
                gm_campaigns::status
                    .eq("Active")
                    .or(gm_campaigns::status.eq("ACTIVE")),
            )
            .count()
            .get_result(&mut conn)
            .map_err(|e| {
                tracing::error!("Failed to fetch active campaigns: {}", e);
                ApiError::InternalServerError("Database error".to_string())
            })?;

        // 3. Total Campaigns: Count of all campaigns
        let total_campaigns: i64 = gm_campaigns::table
            .filter(gm_campaigns::user_id.eq(user_id))
            .count()
            .get_result(&mut conn)
            .map_err(|e| {
                tracing::error!("Failed to fetch total campaigns: {}", e);
                ApiError::InternalServerError("Database error".to_string())
            })?;

        // 4. Interaction Scanned Count: Sum of total_scanned
        let interaction_scanned_count: Option<i64> = gm_campaigns::table
            .filter(gm_campaigns::user_id.eq(user_id))
            .select(sum(gm_campaigns::total_scanned))
            .first(&mut conn)
            .map_err(|e| {
                tracing::error!("Failed to fetch scanned count: {}", e);
                ApiError::InternalServerError("Database error".to_string())
            })?;

        // 5. Total Replied Count: Sum from all platform comment tables with status = 2
        // TikTok comments (using existing joinable)
        let tiktok_replied: i64 = gm_agent_comments::table
            .inner_join(gm_campaigns::table)
            .filter(gm_campaigns::user_id.eq(user_id))
            .filter(gm_agent_comments::status.eq(2))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        // Use raw SQL for other platform comments (simpler than setting up all joins)
        let other_replied: i64 = sql_query(
            r#"
            SELECT COALESCE(
                (SELECT COUNT(*) FROM gm_agent_facebook_comments fc 
                 JOIN gm_campaigns c ON fc.campaign_id = c.id 
                 WHERE c.user_id = $1 AND fc.status = 2), 0) +
                COALESCE(
                (SELECT COUNT(*) FROM gm_agent_instagram_comments ic 
                 JOIN gm_campaigns c ON ic.campaign_id = c.id 
                 WHERE c.user_id = $1 AND ic.status = 2), 0) +
                COALESCE(
                (SELECT COUNT(*) FROM gm_agent_reddit_comments rc 
                 JOIN gm_campaigns c ON rc.campaign_id = c.id 
                 WHERE c.user_id = $1 AND rc.status = 2), 0) +
                COALESCE(
                (SELECT COUNT(*) FROM gm_agent_twitter_comments tc 
                 JOIN gm_campaigns c ON tc.campaign_id = c.id 
                 WHERE c.user_id = $1 AND tc.status = 2), 0)
            AS count
            "#,
        )
        .bind::<diesel::sql_types::Int4, _>(user_id)
        .get_result::<CountResult>(&mut conn)
        .map(|r| r.count)
        .unwrap_or(0);

        let total_replied_count = tiktok_replied + other_replied;

        Ok(OverviewStatsDto {
            total_spent: total_spent.unwrap_or(BigDecimal::from(0)),
            active_campaigns,
            total_campaigns,
            interaction_scanned_count: interaction_scanned_count.unwrap_or(0),
            total_replied_count,
        })
    }

    pub async fn get_performance_stats(
        &self,
        _user_id: i32,
        days: Option<i32>,
    ) -> Result<Vec<PerformanceDataDto>, ApiError> {
        // TODO: Implement real performance data aggregation
        // For now, return mock data
        let num_days = days.unwrap_or(7);
        let mut data = Vec::new();

        for i in 0..num_days {
            data.push(PerformanceDataDto {
                date: format!("2025-12-{}", 29 - i),
                spent: BigDecimal::from(100 + i * 10),
                reach: 5000 + i * 200,
                conversions: 150 + i * 5,
            });
        }

        Ok(data)
    }

    /// Get recent campaigns ordered by updated_at (most recent first)
    pub async fn get_recent_campaigns(
        &self,
        user_id: i32,
        limit: i32,
    ) -> Result<Vec<RecentCampaignDto>, ApiError> {
        let mut conn = self._db.pool.get().map_err(|e| {
            tracing::error!("Failed to get DB connection: {}", e);
            ApiError::InternalServerError("Database error".to_string())
        })?;

        let rows: Vec<RecentCampaignRow> = sql_query(
            r#"
            SELECT 
                c.id,
                c.name,
                c.status,
                COALESCE(p.name, 'Unknown') as platform_name,
                c.actual_consumption,
                c.total_scanned,
                c.updated_at
            FROM gm_campaigns c
            LEFT JOIN gm_platforms p ON c.platform_id = p.id
            WHERE c.user_id = $1
            ORDER BY COALESCE(c.updated_at, c.created_at) DESC
            LIMIT $2
            "#,
        )
        .bind::<Int4, _>(user_id)
        .bind::<Int4, _>(limit)
        .get_results(&mut conn)
        .map_err(|e| {
            tracing::error!("Failed to fetch recent campaigns: {}", e);
            ApiError::InternalServerError("Database error".to_string())
        })?;

        Ok(rows
            .into_iter()
            .map(|row| RecentCampaignDto {
                id: row.id,
                name: row.name,
                status: row.status,
                platform_name: row.platform_name,
                actual_consumption: row.actual_consumption,
                total_scanned: row.total_scanned,
                updated_at: row.updated_at,
            })
            .collect())
    }
}
