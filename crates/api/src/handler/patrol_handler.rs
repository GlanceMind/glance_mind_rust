use axum::extract::{Extension, Path, Query};
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel::result::OptionalExtension;

use crate::api_ok;
use crate::dto::patrol_dto::*;
use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use glance_mind_db::entity::patrol::*;
use glance_mind_db::entity::user::User;
use glance_mind_db::schema::{gm_patrol_account_stats, gm_patrol_reports};

fn db_err(e: impl std::fmt::Display) -> ApiError {
    ApiError::DatabaseError(e.to_string())
}

fn entity_to_report_dto(r: PatrolReportEntity) -> PatrolReportDto {
    PatrolReportDto {
        id: r.id,
        report_id: r.report_id,
        report_type: r.report_type,
        user_id: r.user_id,
        device_id: r.device_id,
        started_at: r.started_at.to_rfc3339(),
        completed_at: r.completed_at.to_rfc3339(),
        total_accounts: r.total_accounts,
        success_count: r.success_count,
        error_count: r.error_count,
        created_at: r.created_at.to_rfc3339(),
        accounts: vec![],
    }
}

fn entity_to_stats_dto(a: PatrolAccountStatsEntity) -> PatrolAccountStatsDto {
    PatrolAccountStatsDto {
        id: a.id,
        report_type: a.report_type,
        social_account_id: a.social_account_id,
        platform_name: a.platform_name,
        username: a.username,
        followers_count: a.followers_count,
        following_count: a.following_count,
        posts_count: a.posts_count,
        total_likes: a.total_likes,
        new_followers: a.new_followers,
        received_likes: a.received_likes,
        received_comments: a.received_comments,
        received_dms: a.received_dms,
        received_shares: a.received_shares,
        received_mentions: a.received_mentions,
        received_friend_requests: a.received_friend_requests,
        unread_total: a.unread_total,
        partial: a.partial,
        error: a.error,
        collected_at: a.collected_at.to_rfc3339(),
    }
}

fn entity_to_brief_dto(r: PatrolReportEntity) -> PatrolReportBriefDto {
    PatrolReportBriefDto {
        report_id: r.report_id,
        report_type: r.report_type,
        total_accounts: r.total_accounts,
        success_count: r.success_count,
        error_count: r.error_count,
        completed_at: r.completed_at.to_rfc3339(),
    }
}

/// GET /patrol/latest
pub async fn get_latest(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
) -> Result<impl IntoResponse, ApiError> {
    let mut conn = state.db.pool.get().map_err(db_err)?;

    let profile_report: Option<PatrolReportEntity> = gm_patrol_reports::table
        .filter(gm_patrol_reports::user_id.eq(user.id))
        .filter(gm_patrol_reports::report_type.eq("profile"))
        .order(gm_patrol_reports::created_at.desc())
        .first(&mut conn)
        .optional()
        .map_err(db_err)?;

    let notif_report: Option<PatrolReportEntity> = gm_patrol_reports::table
        .filter(gm_patrol_reports::user_id.eq(user.id))
        .filter(gm_patrol_reports::report_type.eq("notification"))
        .order(gm_patrol_reports::created_at.desc())
        .first(&mut conn)
        .optional()
        .map_err(db_err)?;

    let report_ids: Vec<String> = [&profile_report, &notif_report]
        .iter()
        .filter_map(|r| r.as_ref().map(|r| r.report_id.clone()))
        .collect();

    let account_stats: Vec<PatrolAccountStatsEntity> = if report_ids.is_empty() {
        vec![]
    } else {
        gm_patrol_account_stats::table
            .filter(gm_patrol_account_stats::report_id.eq_any(&report_ids))
            .load(&mut conn)
            .map_err(db_err)?
    };

    Ok(api_ok!(PatrolLatestDto {
        profile_report: profile_report.map(entity_to_report_dto),
        notification_report: notif_report.map(entity_to_report_dto),
        account_stats: account_stats.into_iter().map(entity_to_stats_dto).collect(),
    }))
}

/// GET /patrol/reports
pub async fn list_reports(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Query(query): Query<PatrolReportsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let mut conn = state.db.pool.get().map_err(db_err)?;
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    let mut count_query = gm_patrol_reports::table
        .filter(gm_patrol_reports::user_id.eq(user.id))
        .into_boxed();

    let mut data_query = gm_patrol_reports::table
        .filter(gm_patrol_reports::user_id.eq(user.id))
        .into_boxed();

    if let Some(ref rt) = query.report_type {
        count_query = count_query.filter(gm_patrol_reports::report_type.eq(rt.clone()));
        data_query = data_query.filter(gm_patrol_reports::report_type.eq(rt.clone()));
    }

    let total: i64 = count_query.count().get_result(&mut conn).map_err(db_err)?;

    let reports: Vec<PatrolReportEntity> = data_query
        .order(gm_patrol_reports::created_at.desc())
        .offset(offset)
        .limit(per_page)
        .load(&mut conn)
        .map_err(db_err)?;

    let report_dtos: Vec<PatrolReportDto> = reports.into_iter().map(entity_to_report_dto).collect();

    Ok(api_ok!(PatrolReportsResponse {
        reports: report_dtos,
        total,
        page,
        per_page,
    }))
}

/// GET /patrol/reports/:report_id
pub async fn get_report(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(report_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let mut conn = state.db.pool.get().map_err(db_err)?;

    let report: PatrolReportEntity = gm_patrol_reports::table
        .filter(gm_patrol_reports::report_id.eq(&report_id))
        .filter(gm_patrol_reports::user_id.eq(user.id))
        .first(&mut conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                ApiError::NotFound(format!("Report {report_id} not found"))
            }
            other => db_err(other),
        })?;

    let accounts: Vec<PatrolAccountStatsEntity> = gm_patrol_account_stats::table
        .filter(gm_patrol_account_stats::report_id.eq(&report_id))
        .load(&mut conn)
        .map_err(db_err)?;

    let mut dto = entity_to_report_dto(report);
    dto.accounts = accounts.into_iter().map(entity_to_stats_dto).collect();

    Ok(api_ok!(dto))
}

/// GET /patrol/accounts/:account_id/trend
pub async fn get_account_trend(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(account_id): Path<i32>,
    Query(query): Query<PatrolTrendQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let mut conn = state.db.pool.get().map_err(db_err)?;

    let days = query.days.unwrap_or(7).min(90);
    let since = chrono::Utc::now() - chrono::Duration::days(days as i64);

    let mut q = gm_patrol_account_stats::table
        .filter(gm_patrol_account_stats::user_id.eq(user.id))
        .filter(gm_patrol_account_stats::social_account_id.eq(account_id))
        .filter(gm_patrol_account_stats::collected_at.ge(since))
        .into_boxed();

    if let Some(ref rt) = query.report_type {
        q = q.filter(gm_patrol_account_stats::report_type.eq(rt.clone()));
    }

    let stats: Vec<PatrolAccountStatsEntity> = q
        .order(gm_patrol_account_stats::collected_at.asc())
        .load(&mut conn)
        .map_err(db_err)?;

    let dtos: Vec<PatrolAccountStatsDto> = stats.into_iter().map(entity_to_stats_dto).collect();

    Ok(api_ok!(dtos))
}

/// GET /patrol/summary
pub async fn get_summary(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
) -> Result<impl IntoResponse, ApiError> {
    let mut conn = state.db.pool.get().map_err(db_err)?;

    let total_reports: i64 = gm_patrol_reports::table
        .filter(gm_patrol_reports::user_id.eq(user.id))
        .count()
        .get_result(&mut conn)
        .map_err(db_err)?;

    let total_accounts: i64 = gm_patrol_account_stats::table
        .filter(gm_patrol_account_stats::user_id.eq(user.id))
        .select(diesel::dsl::count(
            gm_patrol_account_stats::social_account_id,
        ))
        .distinct()
        .get_result(&mut conn)
        .map_err(db_err)?;

    let latest_profile: Option<PatrolReportEntity> = gm_patrol_reports::table
        .filter(gm_patrol_reports::user_id.eq(user.id))
        .filter(gm_patrol_reports::report_type.eq("profile"))
        .order(gm_patrol_reports::created_at.desc())
        .first(&mut conn)
        .optional()
        .map_err(db_err)?;

    let latest_notif: Option<PatrolReportEntity> = gm_patrol_reports::table
        .filter(gm_patrol_reports::user_id.eq(user.id))
        .filter(gm_patrol_reports::report_type.eq("notification"))
        .order(gm_patrol_reports::created_at.desc())
        .first(&mut conn)
        .optional()
        .map_err(db_err)?;

    Ok(api_ok!(PatrolSummaryDto {
        total_reports,
        total_accounts_monitored: total_accounts,
        latest_profile_report: latest_profile.map(entity_to_brief_dto),
        latest_notification_report: latest_notif.map(entity_to_brief_dto),
    }))
}
