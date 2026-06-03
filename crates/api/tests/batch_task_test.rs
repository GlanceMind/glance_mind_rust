//! Module C — DB-backed integration tests for the `gm_ai_batch_creates`
//! ledger and the Diesel dedupe store.
//!
//! These follow the oauth_*_test.rs runtime-skip idiom: when `DATABASE_URL` is
//! unset the tests print a skip notice and return, so the suite still COMPILES
//! and runs cleanly in environments without a database. With a live database
//! they exercise the real table + the real `DieselBatchDedupeStore`, and they
//! fail (RED) against the wrong skeleton stub.
//!
//! Run (DB):
//!   DATABASE_URL=postgres://... cargo test -p glance_mind_api --test batch_task_test

#[cfg(test)]
mod db_tests {
    use diesel::prelude::*;
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::PgConnection;
    use glance_mind_api::service::batch_task_service::{
        BatchDedupeStore, DedupeOutcome, DieselBatchDedupeStore,
    };
    use glance_mind_db::entity::ai_batch_create::{BatchCreate, NewBatchCreate};
    use glance_mind_db::schema::gm_ai_batch_creates;

    type DbPool = Pool<ConnectionManager<PgConnection>>;

    fn maybe_pool() -> Option<DbPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let manager = ConnectionManager::<PgConnection>::new(url);
        Pool::builder().max_size(2).build(manager).ok()
    }

    /// A unique idempotency key per test run so reruns don't collide.
    fn unique_key(prefix: &str) -> String {
        format!("{prefix}-{}", uuid::Uuid::new_v4().simple())
    }

    /// Insert a write-ahead row, read it back, and confirm the CHECK constraint
    /// rejects a status outside {in_progress, completed}.
    #[tokio::test]
    async fn batch_table_accepts_rows() {
        let pool = match maybe_pool() {
            Some(p) => p,
            None => {
                eprintln!("Skipping DB test: DATABASE_URL not set");
                return;
            }
        };
        let mut conn = pool.get().expect("DB connection");

        let key = unique_key("batch-accepts");
        let inserted: BatchCreate = diesel::insert_into(gm_ai_batch_creates::table)
            .values(&NewBatchCreate {
                user_id: 1,
                idempotency_key: key.clone(),
                task_kind: "campaign".to_string(),
            })
            .returning(BatchCreate::as_returning())
            .get_result(&mut conn)
            .expect("insert write-ahead row");

        assert_eq!(inserted.user_id, 1);
        assert_eq!(inserted.idempotency_key, key);
        assert_eq!(inserted.task_kind, "campaign");
        assert_eq!(
            inserted.status, "in_progress",
            "status must default to 'in_progress'"
        );
        assert!(
            inserted.result.is_none(),
            "result must be NULL until the batch completes"
        );

        // Read it back by primary key.
        let fetched: BatchCreate = gm_ai_batch_creates::table
            .find(inserted.id)
            .select(BatchCreate::as_select())
            .first(&mut conn)
            .expect("read row back");
        assert_eq!(fetched.id, inserted.id);

        // The CHECK constraint must reject an invalid status.
        let bad = diesel::update(gm_ai_batch_creates::table.find(inserted.id))
            .set(gm_ai_batch_creates::status.eq("bogus_status"))
            .execute(&mut conn);
        assert!(
            bad.is_err(),
            "status CHECK must reject values outside {{in_progress, completed}}"
        );

        // Cleanup.
        let _ = diesel::delete(gm_ai_batch_creates::table.find(inserted.id)).execute(&mut conn);
    }

    /// Two real `begin`s with the same (user, key) must NOT both insert a fresh
    /// row: the second must observe the first via the UNIQUE(user_id,
    /// idempotency_key) constraint and return Completed/InProgress (i.e. NOT
    /// `Fresh`).
    #[tokio::test]
    async fn dedupe_unique_violation_reads_existing() {
        let pool = match maybe_pool() {
            Some(p) => p,
            None => {
                eprintln!("Skipping DB test: DATABASE_URL not set");
                return;
            }
        };

        let store = DieselBatchDedupeStore::new(pool.clone());
        let user_id = 1;
        let key = unique_key("dedupe");

        let first = store
            .begin(user_id, &key, "campaign")
            .await
            .expect("first begin must succeed");
        assert!(
            matches!(first, DedupeOutcome::Fresh),
            "first begin on a never-seen key must be Fresh, got {first:?}"
        );

        let second = store
            .begin(user_id, &key, "campaign")
            .await
            .expect("second begin must succeed (read existing, not error)");
        assert!(
            matches!(
                second,
                DedupeOutcome::InProgress | DedupeOutcome::Completed(_)
            ),
            "second begin on the same (user, key) must read the existing row \
             (InProgress/Completed), not insert a fresh one; got {second:?}"
        );

        // Cleanup any row(s) created for this key.
        let mut conn = pool.get().expect("DB connection");
        let _ = diesel::delete(
            gm_ai_batch_creates::table
                .filter(gm_ai_batch_creates::user_id.eq(user_id))
                .filter(gm_ai_batch_creates::idempotency_key.eq(&key)),
        )
        .execute(&mut conn);
    }
}
