//! IT6i / IT6j — agent assembly platform-filter tests (M4, production path)
//!
//! These tests call `AgentRepository::get_group_active_accounts` and
//! `AgentRepository::get_group_account_count` directly on the real production
//! code path.  The M4 implementation (commit ccc0738) made `agent_repository`
//! `pub` and added the platform_id filter, so the repository is now callable
//! from external integration-test binaries.
//!
//! Run:
//!   DATABASE_URL=postgres://glancemind:testpassword@localhost:5432/glancemind_test \
//!   cargo test -p glance_mind_api --test agent_group_filter_test -- --nocapture

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use glance_mind_api::repository::agent_repository::AgentRepository;

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

// ─────────────────────────────────────────────────────────────────────────────
// IT6i — facebook group with 1 facebook + 1 reddit account
// ─────────────────────────────────────────────────────────────────────────────

/// IT6i — A facebook group (platform_id=3) contains one facebook account
/// (platform_id=3) and one reddit account (platform_id=1, legacy-dirty).
///
/// `get_group_active_accounts(group_id, 3)` must return ONLY the facebook account.
/// `get_group_account_count(group_id)` must return 2 (total active regardless of
/// platform) — the non-zero total is what distinguishes this from a truly empty
/// group and drives the comment-drop path in agent_service.
#[test]
fn it6i_platform_filter_returns_only_matching_account() {
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

    let repo = AgentRepository::new(pool.clone());

    let filtered = repo
        .get_group_active_accounts(group_id, 3) // platform 3 = facebook
        .expect("get_group_active_accounts failed");
    let filtered_ids: Vec<i32> = filtered.iter().map(|&(id, _, _)| id).collect();

    let total_count = repo
        .get_group_account_count(group_id)
        .expect("get_group_account_count failed");

    delete_accounts(&mut conn, &[fb_acc_id, reddit_acc_id]);
    delete_groups(&mut conn, &[group_id]);

    // Platform-filtered call must return exactly the facebook account.
    assert_eq!(
        filtered.len(),
        1,
        "IT6i: get_group_active_accounts(group={}, platform=3) must return 1 account \
         (only facebook). Got ids={:?}",
        group_id,
        filtered_ids
    );
    assert_eq!(
        filtered_ids,
        vec![fb_acc_id],
        "IT6i: filtered result must contain only fb_acc_id={}. Got {:?}",
        fb_acc_id,
        filtered_ids
    );

    // Total count must be 2 (1 facebook + 1 reddit, both ACTIVE).
    assert_eq!(
        total_count, 2,
        "IT6i: get_group_account_count(group={}) must return 2 (fb+reddit). Got {}",
        group_id, total_count
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// IT6j — reddit-only facebook group: filtered call empty, count=2
// ─────────────────────────────────────────────────────────────────────────────

/// IT6j — A facebook group (platform_id=3) has ONLY reddit accounts.
///
/// `get_group_active_accounts(group_id, 3)` must return 0 accounts.
/// `get_group_account_count(group_id)` must return 2.
///
/// This is the "has accounts but 0 matched" signature that drives the
/// comment-drop path in agent_service (non-empty group, zero platform match
/// → drop, not keep).
#[test]
fn it6j_all_reddit_group_returns_empty_filtered_but_nonzero_total() {
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

    // Seed two reddit accounts in a facebook group
    let group_id = diesel::sql_query(
        "INSERT INTO gm_social_groups (user_id, platform_id, group_name, created_at)
         VALUES (999, 3, $1, NOW()) RETURNING id",
    )
    .bind::<diesel::sql_types::Text, _>(format!("it6j_2reddit_grp_{}", tag))
    .get_result::<IdRow>(&mut conn)
    .expect("group insert")
    .id;

    let reddit_acc1 = diesel::sql_query(
        "INSERT INTO gm_social_accounts (user_id, group_id, platform_id, username,
          cookie, status, daily_max_replies, profile_name)
         VALUES (999, $1, 1, $2, '', 'ACTIVE', 30, $3) RETURNING id",
    )
    .bind::<diesel::sql_types::Integer, _>(group_id)
    .bind::<diesel::sql_types::Text, _>(format!("reddit_a_{}", tag))
    .bind::<diesel::sql_types::Text, _>(format!("RedditProfileA_{}", tag))
    .get_result::<IdRow>(&mut conn)
    .expect("reddit acc1 insert")
    .id;

    let reddit_acc2 = diesel::sql_query(
        "INSERT INTO gm_social_accounts (user_id, group_id, platform_id, username,
          cookie, status, daily_max_replies, profile_name)
         VALUES (999, $1, 1, $2, '', 'ACTIVE', 30, $3) RETURNING id",
    )
    .bind::<diesel::sql_types::Integer, _>(group_id)
    .bind::<diesel::sql_types::Text, _>(format!("reddit_b_{}", tag))
    .bind::<diesel::sql_types::Text, _>(format!("RedditProfileB_{}", tag))
    .get_result::<IdRow>(&mut conn)
    .expect("reddit acc2 insert")
    .id;

    let repo = AgentRepository::new(pool.clone());

    let filtered = repo
        .get_group_active_accounts(group_id, 3) // platform 3 = facebook, none present
        .expect("get_group_active_accounts failed");

    let total_count = repo
        .get_group_account_count(group_id)
        .expect("get_group_account_count failed");

    delete_accounts(&mut conn, &[reddit_acc1, reddit_acc2]);
    delete_groups(&mut conn, &[group_id]);

    // Platform-filtered call must return 0 (no facebook accounts in group).
    assert_eq!(
        filtered.len(),
        0,
        "IT6j: get_group_active_accounts(group={}, platform=3) must return 0 for \
         reddit-only group. Got {:?}",
        group_id,
        filtered.iter().map(|&(id, _, _)| id).collect::<Vec<_>>()
    );

    // Total count must be 2 (2 reddit accounts, both ACTIVE).
    assert_eq!(
        total_count, 2,
        "IT6j: get_group_account_count(group={}) must return 2 for 2-reddit group. \
         Got {}",
        group_id, total_count
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// empty group: filtered call empty + count 0 (legacy passthrough signature)
// ─────────────────────────────────────────────────────────────────────────────

/// empty group — A group with NO accounts at all.
///
/// `get_group_active_accounts(group_id, 3)` must return 0 accounts.
/// `get_group_account_count(group_id)` must return 0.
///
/// In `enforce_daily_limits_inner`, the zero-total case triggers the legacy
/// keep-comment passthrough:
///   `_ => { result.push(comment); continue; }`
/// which is CORRECT for truly empty groups and must be preserved by M4.
/// (M4 differentiates: zero platform-matched BUT non-zero total → drop;
///  zero total → keep.)
#[test]
fn it6j_empty_group_filtered_empty_and_count_zero() {
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

    let group_id = diesel::sql_query(
        "INSERT INTO gm_social_groups (user_id, platform_id, group_name, created_at)
         VALUES (999, 3, $1, NOW()) RETURNING id",
    )
    .bind::<diesel::sql_types::Text, _>(format!("it6j_empty_grp_{}", tag))
    .get_result::<IdRow>(&mut conn)
    .expect("group insert")
    .id;

    let repo = AgentRepository::new(pool.clone());

    let filtered = repo
        .get_group_active_accounts(group_id, 3)
        .expect("get_group_active_accounts failed");

    let total_count = repo
        .get_group_account_count(group_id)
        .expect("get_group_account_count failed");

    delete_groups(&mut conn, &[group_id]);

    assert_eq!(
        filtered.len(),
        0,
        "empty group: get_group_active_accounts must return 0. Got {:?}",
        filtered.iter().map(|&(id, _, _)| id).collect::<Vec<_>>()
    );
    assert_eq!(
        total_count, 0,
        "empty group: get_group_account_count must return 0. Got {}",
        total_count
    );

    eprintln!(
        "empty group confirmed: group {} → 0 filtered, 0 total → \
         enforce_daily_limits_inner keep-comment passthrough applies correctly.",
        group_id
    );
}
