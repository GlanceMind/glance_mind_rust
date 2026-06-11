//! IT6i / IT6j  — agent assembly platform-filter tests (M4, Part B)
//!
//! # Architecture note: why this file cannot call AgentRepository directly
//!
//! `glance_mind_api::repository::agent_repository` is declared `pub(crate)` in
//! `crates/api/src/repository/mod.rs`, so it is invisible to external integration
//! test binaries in `tests/`.
//!
//! `AgentService::enforce_daily_limits_inner` is a private `fn` inside `impl
//! AgentService` and short-circuits with `return comments` when `redis = None`,
//! making platform-filter behavior unobservable without redis.
//!
//! # Proposed seam for M4 implementer
//!
//! To make IT6i/j unit-testable without redis:
//!
//! 1. Change `mod agent_repository` in `repository/mod.rs` to `pub mod agent_repository`
//!    (or add a `pub use` re-export).
//! 2. Change `get_group_active_accounts` signature to accept `platform_id: i32` and
//!    add `.filter(gm_social_accounts::platform_id.eq(platform_id))`.
//! 3. Add `pub(crate)` to `enforce_daily_limits_inner` so tests in this file can call it.
//!    OR add IT6i/j as inline `#[cfg(test)]` tests in `agent_service.rs` (simplest).
//!
//! # What this file tests instead
//!
//! Since we cannot call the Rust functions, we verify the raw DB state that those
//! functions query.  We seed the same fixtures the Rust functions would process and
//! assert the EXPECTED SQL result matches what the current (unfixed) query returns.
//! These assertions are **RED today** — they document the bug by asserting that the
//! current behavior (no filter) is present, and will need to be updated to GREEN
//! once M4 adds the platform filter.
//!
//! # RED/GREEN status today
//!
//! IT6i RED: raw SQL query `SELECT … FROM gm_social_accounts WHERE group_id = $1
//!     AND status = 'ACTIVE'` (equivalent to current get_group_active_accounts)
//!     returns BOTH platform_id=3 and platform_id=1 accounts.
//! IT6j RED: adding `AND platform_id = 3` (proposed M4 filter) returns 0 accounts
//!     for the reddit-only group — documents that the keep-empty fallback in
//!     enforce_daily_limits_inner would then include the comment anyway.
//! IT6j_empty GREEN: truly empty group → 0 accounts regardless of filter.
//!
//! Run:
//!   DATABASE_URL=postgres://glancemind:testpassword@localhost:5432/glancemind_test \
//!   cargo test --test agent_group_filter_test -- --nocapture

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;

type DbPool = Pool<ConnectionManager<PgConnection>>;

fn maybe_pool() -> Option<DbPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let manager = ConnectionManager::<PgConnection>::new(url);
    Pool::builder().max_size(2).build(manager).ok()
}

// ─────────────────────────────────────────────────────────────────────────────
// Typed result structs for sql_query (diesel requires QueryableByName)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(diesel::QueryableByName, Debug)]
struct IdRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    id: i32,
}

#[derive(diesel::QueryableByName, Debug)]
struct AccountRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    id: i32,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    platform_id: i32,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    profile_name: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    daily_max_replies: i32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Seed / cleanup helpers
// ─────────────────────────────────────────────────────────────────────────────

fn seed_mixed_group(conn: &mut PgConnection, user_id: i32, tag: &str) -> Option<(i32, i32, i32)> {
    let group_id = diesel::sql_query(
        "INSERT INTO gm_social_groups (user_id, platform_id, group_name, created_at)
         VALUES ($1, 3, $2, NOW()) RETURNING id",
    )
    .bind::<diesel::sql_types::Integer, _>(user_id)
    .bind::<diesel::sql_types::Text, _>(format!("it6i_grp_{}", tag))
    .get_result::<IdRow>(conn)
    .ok()?
    .id;

    let fb_acc_id = diesel::sql_query(
        "INSERT INTO gm_social_accounts (user_id, group_id, platform_id, username,
          cookie, status, daily_max_replies, profile_name)
         VALUES ($1, $2, 3, $3, '', 'ACTIVE', 30, $4) RETURNING id",
    )
    .bind::<diesel::sql_types::Integer, _>(user_id)
    .bind::<diesel::sql_types::Integer, _>(group_id)
    .bind::<diesel::sql_types::Text, _>(format!("fb_acc_{}", tag))
    .bind::<diesel::sql_types::Text, _>(format!("FBProfile_{}", tag))
    .get_result::<IdRow>(conn)
    .ok()?
    .id;

    let reddit_acc_id = diesel::sql_query(
        "INSERT INTO gm_social_accounts (user_id, group_id, platform_id, username,
          cookie, status, daily_max_replies, profile_name)
         VALUES ($1, $2, 1, $3, '', 'ACTIVE', 30, $4) RETURNING id",
    )
    .bind::<diesel::sql_types::Integer, _>(user_id)
    .bind::<diesel::sql_types::Integer, _>(group_id)
    .bind::<diesel::sql_types::Text, _>(format!("reddit_acc_{}", tag))
    .bind::<diesel::sql_types::Text, _>(format!("RedditProfile_{}", tag))
    .get_result::<IdRow>(conn)
    .ok()?
    .id;

    Some((group_id, fb_acc_id, reddit_acc_id))
}

fn seed_reddit_only_group(conn: &mut PgConnection, user_id: i32, tag: &str) -> Option<(i32, i32)> {
    let group_id = diesel::sql_query(
        "INSERT INTO gm_social_groups (user_id, platform_id, group_name, created_at)
         VALUES ($1, 3, $2, NOW()) RETURNING id",
    )
    .bind::<diesel::sql_types::Integer, _>(user_id)
    .bind::<diesel::sql_types::Text, _>(format!("it6j_grp_{}", tag))
    .get_result::<IdRow>(conn)
    .ok()?
    .id;

    let reddit_acc_id = diesel::sql_query(
        "INSERT INTO gm_social_accounts (user_id, group_id, platform_id, username,
          cookie, status, daily_max_replies, profile_name)
         VALUES ($1, $2, 1, $3, '', 'ACTIVE', 30, $4) RETURNING id",
    )
    .bind::<diesel::sql_types::Integer, _>(user_id)
    .bind::<diesel::sql_types::Integer, _>(group_id)
    .bind::<diesel::sql_types::Text, _>(format!("reddit_only_{}", tag))
    .bind::<diesel::sql_types::Text, _>(format!("RedditOnlyProfile_{}", tag))
    .get_result::<IdRow>(conn)
    .ok()?
    .id;

    Some((group_id, reddit_acc_id))
}

fn delete_accounts(conn: &mut PgConnection, ids: &[i32]) {
    for &id in ids {
        let _ = diesel::sql_query("DELETE FROM gm_social_accounts WHERE id = $1")
            .bind::<diesel::sql_types::Integer, _>(id)
            .execute(conn);
    }
}

fn delete_groups(conn: &mut PgConnection, ids: &[i32]) {
    for &id in ids {
        let _ = diesel::sql_query("DELETE FROM gm_social_groups WHERE id = $1")
            .bind::<diesel::sql_types::Integer, _>(id)
            .execute(conn);
    }
}

/// Raw SQL equivalent of the CURRENT (unfiltered) get_group_active_accounts.
///
/// Mirrors:
///   `gm_social_accounts::table
///     .filter(gm_social_accounts::group_id.eq(group_id))
///     .filter(gm_social_accounts::status.eq("ACTIVE"))
///     .select((id, profile_name, daily_max_replies))
///     .load(&mut conn)`
fn query_current_active_accounts(conn: &mut PgConnection, group_id: i32) -> Vec<AccountRow> {
    diesel::sql_query(
        "SELECT id, platform_id, profile_name, daily_max_replies
         FROM gm_social_accounts
         WHERE group_id = $1 AND status = 'ACTIVE'
         ORDER BY id",
    )
    .bind::<diesel::sql_types::Integer, _>(group_id)
    .load::<AccountRow>(conn)
    .unwrap_or_default()
}

/// Raw SQL equivalent of the PROPOSED M4 filtered get_group_active_accounts.
///
/// Mirrors the expected post-M4 query:
///   `gm_social_accounts::table
///     .filter(gm_social_accounts::group_id.eq(group_id))
///     .filter(gm_social_accounts::status.eq("ACTIVE"))
///     .filter(gm_social_accounts::platform_id.eq(platform_id))  // M4 addition
///     .select((id, profile_name, daily_max_replies))
///     .load(&mut conn)`
fn query_m4_filtered_active_accounts(
    conn: &mut PgConnection,
    group_id: i32,
    platform_id: i32,
) -> Vec<AccountRow> {
    diesel::sql_query(
        "SELECT id, platform_id, profile_name, daily_max_replies
         FROM gm_social_accounts
         WHERE group_id = $1 AND status = 'ACTIVE' AND platform_id = $2
         ORDER BY id",
    )
    .bind::<diesel::sql_types::Integer, _>(group_id)
    .bind::<diesel::sql_types::Integer, _>(platform_id)
    .load::<AccountRow>(conn)
    .unwrap_or_default()
}

// ─────────────────────────────────────────────────────────────────────────────
// IT6i — RED: current query returns BOTH accounts (no platform filter)
// ─────────────────────────────────────────────────────────────────────────────

/// IT6i (RED) — A facebook group contains one facebook account (platform_id=3)
/// and one reddit account (platform_id=1, legacy-dirty).
///
/// The CURRENT `get_group_active_accounts` query (no platform filter) returns both.
/// After M4, `get_group_active_accounts(group_id, platform_id=3)` must return only
/// the facebook account.
///
/// This test asserts TODAY's RED behavior (count=2, both platform_ids present) to
/// document the missing guard.  After M4 the assertion must be updated to count=1.
#[test]
fn it6i_current_query_returns_both_platforms_red() {
    let pool = match maybe_pool() {
        Some(p) => p,
        None => {
            eprintln!("Skipping it6i: DATABASE_URL not set");
            return;
        }
    };
    let mut conn = pool.get().expect("DB connection");
    let tag = uuid::Uuid::new_v4().simple().to_string();
    let tag = &tag[..8];

    let (group_id, fb_acc_id, reddit_acc_id) = match seed_mixed_group(&mut conn, 999, tag) {
        Some(ids) => ids,
        None => {
            eprintln!("Skipping it6i: seed failed");
            return;
        }
    };

    // Current unfiltered query (what get_group_active_accounts does today)
    let current = query_current_active_accounts(&mut conn, group_id);
    let current_ids: Vec<i32> = current.iter().map(|r| r.id).collect();
    let current_platforms: Vec<i32> = current.iter().map(|r| r.platform_id).collect();

    // M4 filtered query (what get_group_active_accounts should do after M4)
    let m4_filtered = query_m4_filtered_active_accounts(&mut conn, group_id, 3); // platform 3 = facebook
    let m4_ids: Vec<i32> = m4_filtered.iter().map(|r| r.id).collect();

    delete_accounts(&mut conn, &[fb_acc_id, reddit_acc_id]);
    delete_groups(&mut conn, &[group_id]);

    // ── RED assertion: current behavior ──────────────────────────────────────
    // Today the unfiltered query returns BOTH accounts.
    // After M4 this test must be updated: the production function should return
    // only 1 account (the facebook one).
    assert_eq!(
        current.len(),
        2,
        "IT6i RED: get_group_active_accounts (current, unfiltered) must return 2 accounts \
         for a group with fb+reddit accounts. Got {:?} platforms, ids={:?}. \
         After M4 this assertion must flip to 1 (only facebook, platform_id=3).",
        current_platforms,
        current_ids
    );
    assert!(
        current_ids.contains(&fb_acc_id),
        "IT6i: fb account {} must be in current result {:?}",
        fb_acc_id,
        current_ids
    );
    assert!(
        current_ids.contains(&reddit_acc_id),
        "IT6i RED: reddit account {} is currently in result (no platform filter). \
         ids={:?}",
        reddit_acc_id,
        current_ids
    );

    // ── Document M4 expected behavior ────────────────────────────────────────
    // The M4 filtered query should return ONLY the facebook account.
    assert_eq!(
        m4_filtered.len(),
        1,
        "IT6i (M4 expected): filtered query for platform_id=3 must return 1 account. \
         Got ids={:?}",
        m4_ids
    );
    assert_eq!(
        m4_ids,
        vec![fb_acc_id],
        "IT6i (M4 expected): filtered result must contain only fb_acc_id={}. \
         Got {:?}",
        fb_acc_id,
        m4_ids
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// IT6j — RED: reddit-only group, current query picks wrong-platform account
// ─────────────────────────────────────────────────────────────────────────────

/// IT6j (RED) — A facebook group has ONLY a reddit account.
///
/// Current behavior:
///   - `get_group_active_accounts` returns 1 account (reddit, wrong platform)
///   - enforce_daily_limits_inner sees a non-empty accounts list
///   - Comment is included in assembly result (keep-comment via quota/fallback)
///
/// After M4:
///   - `get_group_active_accounts(group_id, platform_id=3)` returns 0 accounts
///   - enforce_daily_limits_inner hits the empty-accounts branch and MUST drop
///     the comment (M4 must add: "empty due to platform filter → drop, not keep")
///
/// This test documents today's wrong behavior via the DB query.
#[test]
fn it6j_current_query_returns_wrong_platform_account_red() {
    let pool = match maybe_pool() {
        Some(p) => p,
        None => {
            eprintln!("Skipping it6j: DATABASE_URL not set");
            return;
        }
    };
    let mut conn = pool.get().expect("DB connection");
    let tag = uuid::Uuid::new_v4().simple().to_string();
    let tag = &tag[..8];

    let (group_id, reddit_acc_id) = match seed_reddit_only_group(&mut conn, 999, tag) {
        Some(ids) => ids,
        None => {
            eprintln!("Skipping it6j: seed failed");
            return;
        }
    };

    let current = query_current_active_accounts(&mut conn, group_id);
    let current_ids: Vec<i32> = current.iter().map(|r| r.id).collect();
    let current_platforms: Vec<i32> = current.iter().map(|r| r.platform_id).collect();

    let m4_filtered = query_m4_filtered_active_accounts(&mut conn, group_id, 3); // platform 3 = facebook

    delete_accounts(&mut conn, &[reddit_acc_id]);
    delete_groups(&mut conn, &[group_id]);

    // ── RED assertion: current behavior ──────────────────────────────────────
    // Today: the reddit account IS returned (no platform filter).
    // The keep-comment-on-empty fallback in enforce_daily_limits_inner doesn't
    // fire because accounts.is_empty() is false — the comment is included via
    // the normal quota path (or short-circuits with redis=None).
    assert_eq!(
        current.len(),
        1,
        "IT6j RED: get_group_active_accounts (current, unfiltered) returns 1 account \
         for reddit-only facebook group. Got platforms={:?}, ids={:?}. \
         After M4 the platform-filtered query must return 0 for facebook campaign.",
        current_platforms,
        current_ids
    );
    assert!(
        current_ids.contains(&reddit_acc_id),
        "IT6j RED: reddit account {} currently in result. ids={:?}",
        reddit_acc_id,
        current_ids
    );
    assert_eq!(
        current[0].platform_id, 1,
        "IT6j RED: the returned account has platform_id=1 (reddit), not 3 (facebook). \
         This is the wrong-platform leak that M4 must fix."
    );

    // ── Document M4 expected behavior ────────────────────────────────────────
    // After M4, the filtered query for facebook (platform_id=3) returns 0 accounts.
    // The service must then DROP the comment (not use the keep-empty passthrough).
    assert_eq!(
        m4_filtered.len(),
        0,
        "IT6j (M4 expected): filtered query for platform_id=3 must return 0 accounts \
         when group has only reddit accounts. Got {:?}",
        m4_filtered.iter().map(|r| r.id).collect::<Vec<_>>()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// IT6j_empty — GREEN baseline: truly empty group returns 0 accounts
// ─────────────────────────────────────────────────────────────────────────────

/// IT6j_empty (GREEN baseline) — A group with NO accounts at all.
///
/// Both the current and M4 filtered queries return 0 accounts.
/// In `enforce_daily_limits_inner`, this triggers the empty-accounts branch:
///   `_ => { result.push(comment); continue; }`
/// which keeps the comment — CORRECT behavior for truly empty groups.
///
/// M4 must NOT change this: empty group (0 accounts total) → keep comment.
/// The distinction M4 must add is: group with 0 PLATFORM-MATCHED accounts
/// (but non-zero total accounts) → DROP comment.
#[test]
fn it6j_empty_group_zero_accounts_passthrough_green() {
    let pool = match maybe_pool() {
        Some(p) => p,
        None => {
            eprintln!("Skipping it6j_empty: DATABASE_URL not set");
            return;
        }
    };
    let mut conn = pool.get().expect("DB connection");
    let tag = uuid::Uuid::new_v4().simple().to_string();
    let tag = &tag[..8];

    // Group with NO accounts
    let group_id = diesel::sql_query(
        "INSERT INTO gm_social_groups (user_id, platform_id, group_name, created_at)
         VALUES (999, 3, $1, NOW()) RETURNING id",
    )
    .bind::<diesel::sql_types::Text, _>(format!("it6j_empty_grp_{}", tag))
    .get_result::<IdRow>(&mut conn)
    .expect("group insert")
    .id;

    let current = query_current_active_accounts(&mut conn, group_id);
    let m4_filtered = query_m4_filtered_active_accounts(&mut conn, group_id, 3);

    delete_groups(&mut conn, &[group_id]);

    // GREEN: truly empty group → 0 accounts from both queries
    assert_eq!(
        current.len(),
        0,
        "IT6j_empty GREEN: truly empty group must return 0 accounts. Got {:?}",
        current.len()
    );
    assert_eq!(
        m4_filtered.len(),
        0,
        "IT6j_empty GREEN: M4 filtered query also returns 0 for empty group."
    );

    // Document the passthrough contract:
    // enforce_daily_limits_inner line ~187-190:
    //   let accounts = match group_accounts.get(&group_id) {
    //       Some(a) if !a.is_empty() => a,
    //       _ => { result.push(comment); continue; }   ← keep comment
    //   };
    // With 0 accounts: comment is kept. This is correct for truly empty groups
    // and must be preserved by M4.
    eprintln!(
        "IT6j_empty GREEN confirmed: group {} has 0 accounts → \
         enforce_daily_limits_inner keep-comment fallback applies correctly.",
        group_id
    );
}
