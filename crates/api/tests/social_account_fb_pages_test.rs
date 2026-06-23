//! Social-account Facebook-pages CRUD integration tests (module M4).
//!
//! Exercises the full chain — DTO → service → diesel → DB → to_dto — for the
//! `fb_pages_id` field against a real Postgres, so a silent field-drop at any
//! layer is caught (the `platform_id` no-op class). Covers R004 (read),
//! R014 (every CRUD op), R015 (silent-drop regression) and the A-PLAT
//! permissive-non-FB pin.
//!
//! Skipped silently when DATABASE_URL is unset (mirrors `oauth_token_test.rs`):
//!   DATABASE_URL=postgres://... cargo test -p glance_mind_api --test social_account_fb_pages_test

use std::sync::Arc;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;

use glance_mind_api::config::database::Database;
use glance_mind_api::dto::fb_page::{normalize_pages, serialize_pages, FbPage};
use glance_mind_api::dto::social_account_dto::{
    AccountListRequest, CreateSocialAccountDto, UpdateSocialAccountDto,
};
use glance_mind_api::service::social_account_service::SocialAccountService;

const FACEBOOK: i32 = 3;
const TIKTOK: i32 = 2;

#[derive(diesel::QueryableByName)]
struct IdRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    id: i32,
}

/// Build a service + a valid user_id from DATABASE_URL, or `None` to skip.
fn maybe_setup() -> Option<(SocialAccountService, Arc<Database>, i32)> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let manager = ConnectionManager::<PgConnection>::new(url);
    let pool = Pool::builder().build(manager).ok()?;
    let db = Arc::new(Database { pool });
    let user_id = {
        let mut conn = db.pool.get().ok()?;
        diesel::sql_query("SELECT id FROM gm_users ORDER BY id LIMIT 1")
            .get_result::<IdRow>(&mut conn)
            .ok()?
            .id
    };
    let svc = SocialAccountService::new(&db);
    Some((svc, db, user_id))
}

fn fp(id: &str, name: Option<&str>) -> FbPage {
    FbPage {
        id: id.to_string(),
        name: name.map(String::from),
    }
}

fn create_dto(platform_id: i32, pages: Option<Vec<FbPage>>) -> CreateSocialAccountDto {
    CreateSocialAccountDto {
        platform_id,
        username: format!("fbpages_test_{}", uuid::Uuid::new_v4()),
        cookie: None,
        proxy_url: None,
        daily_max_replies: None,
        device_id: None,
        profile_name: None,
        fb_pages_id: pages,
    }
}

/// Read the persisted account back via the service (entity carries the raw
/// `fb_pages_id` column string).
async fn raw_column(svc: &SocialAccountService, id: i32, user_id: i32) -> Option<String> {
    svc.get_account_by_id(id, user_id)
        .await
        .expect("account fetch")
        .fb_pages_id
}

// ---------------------------------------------------------------------------
// R014 / R009 — Create with 0 / 1 / many pages, round-trips faithfully
// ---------------------------------------------------------------------------
#[tokio::test]
async fn create_account_with_pages_roundtrip() {
    let Some((svc, _db, user_id)) = maybe_setup() else {
        eprintln!("Skipping: DATABASE_URL not set");
        return;
    };

    // many (2 pages, one named + one unnamed)
    let dto = create_dto(
        FACEBOOK,
        Some(vec![
            fp("100082341853837", Some("Shop")),
            fp("61556000000000", None),
        ]),
    );
    let created = svc.create_account(user_id, dto).await.expect("create many");
    assert_eq!(created.fb_pages_id.len(), 2);
    assert_eq!(created.fb_pages_id[0], fp("100082341853837", Some("Shop")));
    assert_eq!(created.fb_pages_id[1], fp("61556000000000", None));

    // one
    let one = svc
        .create_account(user_id, create_dto(FACEBOOK, Some(vec![fp("123", None)])))
        .await
        .expect("create one");
    assert_eq!(one.fb_pages_id, vec![fp("123", None)]);

    // zero (None ⇒ empty array in response, NULL in DB)
    let zero = svc
        .create_account(user_id, create_dto(FACEBOOK, None))
        .await
        .expect("create zero");
    assert!(zero.fb_pages_id.is_empty());
    assert_eq!(raw_column(&svc, zero.id, user_id).await, None);

    for id in [created.id, one.id, zero.id] {
        svc.delete_account(id, user_id).await.ok();
    }
}

// ---------------------------------------------------------------------------
// R004 — list + detail both surface the pages array
// ---------------------------------------------------------------------------
#[tokio::test]
async fn list_and_detail_include_pages() {
    let Some((svc, _db, user_id)) = maybe_setup() else {
        return;
    };
    let dto = create_dto(FACEBOOK, Some(vec![fp("100082341853837", Some("Shop"))]));
    let username = dto.username.clone();
    let created = svc.create_account(user_id, dto).await.expect("create");

    // list (DTO path)
    let page = svc
        .list_accounts(
            user_id,
            AccountListRequest {
                page: 1,
                page_size: 50,
                group_id: None,
                username: Some(username),
                platform_id: None,
                status: None,
                device_id: None,
            },
        )
        .await
        .expect("list");
    let listed = page
        .list
        .iter()
        .find(|a| a.id == created.id)
        .expect("account in list");
    assert_eq!(
        listed.fb_pages_id,
        vec![fp("100082341853837", Some("Shop"))]
    );

    // detail (entity path) carries the raw column
    let entity = svc
        .get_account_by_id(created.id, user_id)
        .await
        .expect("detail");
    assert_eq!(
        entity.fb_pages_id.as_deref(),
        Some(r#"[{"id":"100082341853837","name":"Shop"}]"#)
    );

    svc.delete_account(created.id, user_id).await.ok();
}

// ---------------------------------------------------------------------------
// R014 — Update: add / remove / clear / leave-unchanged
// ---------------------------------------------------------------------------
fn update_pages(pages: Option<Vec<FbPage>>) -> UpdateSocialAccountDto {
    UpdateSocialAccountDto {
        platform_id: None,
        username: None,
        cookie: None,
        proxy_url: None,
        status: None,
        group_id: None,
        daily_max_replies: None,
        device_id: None,
        profile_name: None,
        fb_pages_id: pages,
    }
}

#[tokio::test]
async fn update_add_page() {
    let Some((svc, _db, user_id)) = maybe_setup() else {
        return;
    };
    let created = svc
        .create_account(user_id, create_dto(FACEBOOK, Some(vec![fp("1", None)])))
        .await
        .expect("create");
    let updated = svc
        .update_account(
            created.id,
            user_id,
            update_pages(Some(vec![fp("1", None), fp("2", Some("Two"))])),
        )
        .await
        .expect("update add");
    assert_eq!(
        updated.fb_pages_id,
        vec![fp("1", None), fp("2", Some("Two"))]
    );
    svc.delete_account(created.id, user_id).await.ok();
}

#[tokio::test]
async fn update_remove_page() {
    let Some((svc, _db, user_id)) = maybe_setup() else {
        return;
    };
    let created = svc
        .create_account(
            user_id,
            create_dto(
                FACEBOOK,
                Some(vec![fp("1", None), fp("2", None), fp("3", None)]),
            ),
        )
        .await
        .expect("create");
    let updated = svc
        .update_account(created.id, user_id, update_pages(Some(vec![fp("2", None)])))
        .await
        .expect("update remove");
    assert_eq!(updated.fb_pages_id, vec![fp("2", None)]);
    svc.delete_account(created.id, user_id).await.ok();
}

#[tokio::test]
async fn update_clear_all_pages() {
    let Some((svc, _db, user_id)) = maybe_setup() else {
        return;
    };
    let created = svc
        .create_account(
            user_id,
            create_dto(FACEBOOK, Some(vec![fp("1", None), fp("2", None)])),
        )
        .await
        .expect("create");
    // explicit empty array ⇒ clear (NULL in DB)
    let updated = svc
        .update_account(created.id, user_id, update_pages(Some(vec![])))
        .await
        .expect("update clear");
    assert!(updated.fb_pages_id.is_empty());
    assert_eq!(raw_column(&svc, created.id, user_id).await, None);
    svc.delete_account(created.id, user_id).await.ok();
}

#[tokio::test]
async fn update_absent_leaves_pages_unchanged() {
    let Some((svc, _db, user_id)) = maybe_setup() else {
        return;
    };
    let created = svc
        .create_account(
            user_id,
            create_dto(FACEBOOK, Some(vec![fp("1", Some("Keep")), fp("2", None)])),
        )
        .await
        .expect("create");
    // update something else (username) WITHOUT fb_pages_id ⇒ pages survive
    let mut dto = update_pages(None);
    dto.username = Some(format!("renamed_{}", uuid::Uuid::new_v4()));
    let updated = svc
        .update_account(created.id, user_id, dto)
        .await
        .expect("update other");
    assert_eq!(
        updated.fb_pages_id,
        vec![fp("1", Some("Keep")), fp("2", None)]
    );
    svc.delete_account(created.id, user_id).await.ok();
}

// ---------------------------------------------------------------------------
// R014 — Delete removes the account (and its in-row pages)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn delete_account_drops_pages() {
    let Some((svc, _db, user_id)) = maybe_setup() else {
        return;
    };
    let created = svc
        .create_account(
            user_id,
            create_dto(FACEBOOK, Some(vec![fp("100082341853837", Some("Shop"))])),
        )
        .await
        .expect("create");
    svc.delete_account(created.id, user_id)
        .await
        .expect("delete");
    // the row (and its pages) is gone
    assert!(svc.get_account_by_id(created.id, user_id).await.is_err());
}

// ---------------------------------------------------------------------------
// R015 / INV1 — silent-drop regression guard: raw persisted == serialize(normalize(sent)) AND non-empty
// ---------------------------------------------------------------------------
#[tokio::test]
async fn regression_no_silent_drop() {
    let Some((svc, _db, user_id)) = maybe_setup() else {
        return;
    };
    let sent = vec![
        fp("100082341853837", Some("Shop")),
        fp("61556000000000", None),
    ];
    let created = svc
        .create_account(user_id, create_dto(FACEBOOK, Some(sent.clone())))
        .await
        .expect("create");

    let raw = raw_column(&svc, created.id, user_id).await;
    let expected = serialize_pages(&normalize_pages(sent));
    // (a) byte-exact round-trip through the real column
    assert_eq!(raw, expected);
    // (b) and it is genuinely non-empty (so a "drops-everything" bug can't pass)
    let raw = raw.expect("persisted value present, not NULL");
    assert!(raw.contains("100082341853837"));
    assert!(raw.contains("61556000000000"));

    svc.delete_account(created.id, user_id).await.ok();
}

// ---------------------------------------------------------------------------
// A-PLAT — permissive: pages persist on a non-Facebook account (no server gate)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn create_non_facebook_account_with_pages_persists() {
    let Some((svc, _db, user_id)) = maybe_setup() else {
        return;
    };
    let created = svc
        .create_account(
            user_id,
            create_dto(TIKTOK, Some(vec![fp("100082341853837", Some("Shop"))])),
        )
        .await
        .expect("create non-fb with pages");
    // deliberate permissive behavior: backend stores pages regardless of platform
    assert_eq!(
        created.fb_pages_id,
        vec![fp("100082341853837", Some("Shop"))]
    );
    svc.delete_account(created.id, user_id).await.ok();
}
