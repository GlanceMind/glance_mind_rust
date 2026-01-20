use crate::config::database::Database;
use crate::dto::wallet_dto::{OverviewStatsDto, PerformanceDataDto};
use crate::error::api_error::ApiError;
use glance_mind_db::schema::{gm_agent_comments, gm_campaigns};
use bigdecimal::BigDecimal;
use diesel::dsl::sum;
use diesel::prelude::*;
use std::sync::Arc;

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

        // 1. Total Budget: Sum of budget_cap for all user campaigns
        let total_budget: Option<BigDecimal> = gm_campaigns::table
            .filter(gm_campaigns::user_id.eq(user_id))
            .select(sum(gm_campaigns::budget_cap))
            .first(&mut conn)
            .map_err(|e| {
                tracing::error!("Failed to fetch total budget: {}", e);
                ApiError::InternalServerError("Database error".to_string())
            })?;

        // 2. Active Campaigns: Count of campaigns with status 'Active'
        let active_campaigns: i64 = gm_campaigns::table
            .filter(gm_campaigns::user_id.eq(user_id))
            // Assuming 'Active' is the status string. Case-sensitive.
            .filter(gm_campaigns::status.eq("Active"))
            .count()
            .get_result(&mut conn)
            .map_err(|e| {
                tracing::error!("Failed to fetch active campaigns: {}", e);
                ApiError::InternalServerError("Database error".to_string())
            })?;

        // 3. Interaction Scanned Count: Sum of total_scanned
        // total_scanned is Int4, diesel sum returns Option<i64> for Int4 column on PG
        let interaction_scanned_count: Option<i64> = gm_campaigns::table
            .filter(gm_campaigns::user_id.eq(user_id))
            .select(sum(gm_campaigns::total_scanned))
            .first(&mut conn)
            .map_err(|e| {
                tracing::error!("Failed to fetch scanned count: {}", e);
                ApiError::InternalServerError("Database error".to_string())
            })?;

        // 4. Relevant Comments Count: Count of all agent comments linked to user's campaigns
        let relevant_comments_count: i64 = gm_agent_comments::table
            .inner_join(gm_campaigns::table)
            .filter(gm_campaigns::user_id.eq(user_id))
            .count()
            .get_result(&mut conn)
            .map_err(|e| {
                tracing::error!("Failed to fetch relevant comments count: {}", e);
                ApiError::InternalServerError("Database error".to_string())
            })?;

        // 5. Replied Count: Count of agent comments with status = 2 (Replied)
        let replied_count: i64 = gm_agent_comments::table
            .inner_join(gm_campaigns::table)
            .filter(gm_campaigns::user_id.eq(user_id))
            .filter(gm_agent_comments::status.eq(2))
            .count()
            .get_result(&mut conn)
            .map_err(|e| {
                tracing::error!("Failed to fetch replied count: {}", e);
                ApiError::InternalServerError("Database error".to_string())
            })?;

        Ok(OverviewStatsDto {
            total_budget: total_budget.unwrap_or(BigDecimal::from(0)),
            active_campaigns,
            interaction_scanned_count: interaction_scanned_count.unwrap_or(0),
            relevant_comments_count,
            replied_count,
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
}
