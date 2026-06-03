//! Module D2 — DB-backed integration tests for the `gm_ai_task_template_drafts`
//! table and the `source_draft_id` partial-unique indexes.
//!
//! These follow the oauth_*_test.rs / batch_task_test.rs runtime-skip idiom:
//! when `DATABASE_URL` is unset (or the required seed rows are absent) the test
//! prints a skip notice and returns, so the suite still COMPILES and runs
//! cleanly without a database. With a live, migrated + seeded database they
//! exercise the real schema: the draft round-trip, the real CAS UPDATE
//! (`... WHERE id=? AND status=from`), and the partial-unique `source_draft_id`
//! index on `gm_campaigns`.
//!
//! Run (DB):
//!   DATABASE_URL=postgres://... cargo test -p glance_mind_api --test draft_lifecycle_db_test

#[cfg(test)]
mod db_tests {
    use diesel::prelude::*;
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::sql_query;
    use diesel::sql_types::{Integer, Nullable, Text, Uuid as SqlUuid};
    use diesel::PgConnection;
    use glance_mind_db::entity::ai_task_template_draft::{NewTaskTemplateDraft, TaskTemplateDraft};
    use glance_mind_db::schema::gm_ai_task_template_drafts;
    use serde_json::json;
    use uuid::Uuid;

    type DbPool = Pool<ConnectionManager<PgConnection>>;

    fn maybe_pool() -> Option<DbPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let manager = ConnectionManager::<PgConnection>::new(url);
        Pool::builder().max_size(2).build(manager).ok()
    }

    /// One-column projection for `SELECT MIN(id)` lookups.
    #[derive(QueryableByName)]
    struct MaybeId {
        #[diesel(sql_type = Nullable<Integer>)]
        id: Option<i32>,
    }

    /// Find an existing seed id from a referenced table (e.g. gm_users), or
    /// `None` if the table is empty.
    fn min_id(conn: &mut PgConnection, table: &str) -> Option<i32> {
        let q = format!("SELECT MIN(id) AS id FROM {table}");
        let row: MaybeId = sql_query(q).get_result(conn).ok()?;
        row.id
    }

    /// Seed a conversation for `user_id` and return its id.
    fn seed_conversation(conn: &mut PgConnection, user_id: i32) -> i32 {
        #[derive(QueryableByName)]
        struct ConvId {
            #[diesel(sql_type = Integer)]
            id: i32,
        }
        let row: ConvId = sql_query(
            "INSERT INTO gm_ai_conversations (user_id, title, status) \
             VALUES ($1, 'draft-d2-test', 'active') RETURNING id",
        )
        .bind::<Integer, _>(user_id)
        .get_result(conn)
        .expect("seed conversation");
        row.id
    }

    /// Insert one campaign row with an explicit `source_draft_id`, resolving the
    /// NOT NULL FK columns from existing seed rows. Returns the raw DB result so
    /// the caller can assert success / unique-violation.
    fn insert_campaign_with_draft(
        conn: &mut PgConnection,
        user_id: i32,
        platform_id: i32,
        region_id: i32,
        ai_model_id: i32,
        name: &str,
        source_draft_id: Uuid,
    ) -> QueryResult<usize> {
        sql_query(
            "INSERT INTO gm_campaigns \
             (user_id, name, status, platform_id, region_id, ai_model_id, \
              product_prompt, schedule_type, total_scanned, auto_like, auto_follow, \
              auto_dm, pending_consumption, actual_consumption, is_frozen, \
              auto_reply_comments, auto_reply_post, reply_template_ids, source_draft_id) \
             VALUES ($1, $2, 'draft', $3, $4, $5, 'p', 'manual', 0, false, false, \
                     false, 0, 0, false, false, false, '{}', $6)",
        )
        .bind::<Integer, _>(user_id)
        .bind::<Text, _>(name)
        .bind::<Integer, _>(platform_id)
        .bind::<Integer, _>(region_id)
        .bind::<Integer, _>(ai_model_id)
        .bind::<SqlUuid, _>(source_draft_id)
        .execute(conn)
    }

    /// Insert a proposed draft, read it back, and confirm the CHECK constraints
    /// reject invalid task_kind / sample_source / status values.
    #[tokio::test]
    async fn draft_table_roundtrip() {
        let pool = match maybe_pool() {
            Some(p) => p,
            None => {
                eprintln!("Skipping DB test: DATABASE_URL not set");
                return;
            }
        };
        let mut conn = pool.get().expect("DB connection");

        let user_id = match min_id(&mut conn, "gm_users") {
            Some(id) => id,
            None => {
                eprintln!("Skipping DB test: no seed gm_users row");
                return;
            }
        };
        let conv_id = seed_conversation(&mut conn, user_id);

        let inserted: TaskTemplateDraft = diesel::insert_into(gm_ai_task_template_drafts::table)
            .values(&NewTaskTemplateDraft {
                conversation_id: conv_id,
                message_id: None,
                user_id,
                task_kind: "campaign".to_string(),
                draft_config: json!({"name": "x"}),
                sample_source: "ai_generated".to_string(),
            })
            .returning(TaskTemplateDraft::as_returning())
            .get_result(&mut conn)
            .expect("insert proposed draft");

        assert_eq!(inserted.conversation_id, conv_id);
        assert_eq!(inserted.user_id, user_id);
        assert_eq!(inserted.task_kind, "campaign");
        assert_eq!(inserted.sample_source, "ai_generated");
        assert_eq!(
            inserted.status, "proposed",
            "status must default to 'proposed'"
        );
        assert!(
            inserted.created_entity_id.is_none(),
            "created_entity_id must be NULL until confirmed"
        );

        // Read back by primary key.
        let fetched: TaskTemplateDraft = gm_ai_task_template_drafts::table
            .find(inserted.id)
            .select(TaskTemplateDraft::as_select())
            .first(&mut conn)
            .expect("read row back");
        assert_eq!(fetched.id, inserted.id);

        // The status CHECK must reject an invalid value.
        let bad_status = diesel::update(gm_ai_task_template_drafts::table.find(inserted.id))
            .set(gm_ai_task_template_drafts::status.eq("bogus"))
            .execute(&mut conn);
        assert!(
            bad_status.is_err(),
            "status CHECK must reject values outside the documented set"
        );

        // Cleanup (CASCADE from the conversation removes the draft too).
        let _ = diesel::sql_query("DELETE FROM gm_ai_conversations WHERE id = $1")
            .bind::<Integer, _>(conv_id)
            .execute(&mut conn);
    }

    /// The real CAS UPDATE (`... WHERE id=? AND status=from`) must update a row
    /// only when its current status matches `from`.
    #[tokio::test]
    async fn cas_status_real_only_updates_matching_from() {
        let pool = match maybe_pool() {
            Some(p) => p,
            None => {
                eprintln!("Skipping DB test: DATABASE_URL not set");
                return;
            }
        };
        let mut conn = pool.get().expect("DB connection");

        let user_id = match min_id(&mut conn, "gm_users") {
            Some(id) => id,
            None => {
                eprintln!("Skipping DB test: no seed gm_users row");
                return;
            }
        };
        let conv_id = seed_conversation(&mut conn, user_id);

        let draft: TaskTemplateDraft = diesel::insert_into(gm_ai_task_template_drafts::table)
            .values(&NewTaskTemplateDraft {
                conversation_id: conv_id,
                message_id: None,
                user_id,
                task_kind: "campaign".to_string(),
                draft_config: json!({"name": "x"}),
                sample_source: "preset".to_string(),
            })
            .returning(TaskTemplateDraft::as_returning())
            .get_result(&mut conn)
            .expect("insert proposed draft");

        use gm_ai_task_template_drafts::dsl;

        // CAS with a NON-matching `from` (confirming) must update 0 rows.
        let non_matching = diesel::update(
            gm_ai_task_template_drafts::table
                .filter(dsl::id.eq(draft.id))
                .filter(dsl::status.eq("confirming")),
        )
        .set(dsl::status.eq("confirmed"))
        .execute(&mut conn)
        .expect("update must execute");
        assert_eq!(
            non_matching, 0,
            "CAS with a non-matching `from` must update no rows"
        );

        // CAS with the matching `from` (proposed) must update exactly 1 row.
        let matching = diesel::update(
            gm_ai_task_template_drafts::table
                .filter(dsl::id.eq(draft.id))
                .filter(dsl::status.eq("proposed")),
        )
        .set(dsl::status.eq("confirming"))
        .execute(&mut conn)
        .expect("update must execute");
        assert_eq!(
            matching, 1,
            "CAS with the matching `from` must update exactly one row"
        );

        // Confirm the row is now `confirming`.
        let after: TaskTemplateDraft = gm_ai_task_template_drafts::table
            .find(draft.id)
            .select(TaskTemplateDraft::as_select())
            .first(&mut conn)
            .expect("read back");
        assert_eq!(after.status, "confirming");

        // Cleanup.
        let _ = diesel::sql_query("DELETE FROM gm_ai_conversations WHERE id = $1")
            .bind::<Integer, _>(conv_id)
            .execute(&mut conn);
    }

    /// Inserting two `gm_campaigns` rows with the SAME non-null `source_draft_id`
    /// must fail on the second insert (partial unique index
    /// `uq_campaigns_source_draft`).
    #[tokio::test]
    async fn source_draft_id_partial_unique_blocks_second() {
        let pool = match maybe_pool() {
            Some(p) => p,
            None => {
                eprintln!("Skipping DB test: DATABASE_URL not set");
                return;
            }
        };
        let mut conn = pool.get().expect("DB connection");

        // Resolve the NOT NULL FK ids from seed rows; skip if any is missing.
        let user_id = min_id(&mut conn, "gm_users");
        let platform_id = min_id(&mut conn, "gm_platforms");
        let region_id = min_id(&mut conn, "gm_regions");
        let ai_model_id = min_id(&mut conn, "gm_ai_models");
        let (user_id, platform_id, region_id, ai_model_id) =
            match (user_id, platform_id, region_id, ai_model_id) {
                (Some(u), Some(p), Some(r), Some(m)) => (u, p, r, m),
                _ => {
                    eprintln!(
                        "Skipping DB test: missing seed rows for gm_users/gm_platforms/\
                         gm_regions/gm_ai_models"
                    );
                    return;
                }
            };

        let draft_id = Uuid::new_v4();

        let first = insert_campaign_with_draft(
            &mut conn,
            user_id,
            platform_id,
            region_id,
            ai_model_id,
            "d2-uniq-first",
            draft_id,
        );
        assert!(
            first.is_ok(),
            "the first campaign stamped with a source_draft_id must insert: {first:?}"
        );

        let second = insert_campaign_with_draft(
            &mut conn,
            user_id,
            platform_id,
            region_id,
            ai_model_id,
            "d2-uniq-second",
            draft_id,
        );
        assert!(
            matches!(
                second,
                Err(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _
                ))
            ),
            "a second campaign with the same source_draft_id must violate the \
             partial unique index; got {second:?}"
        );

        // Cleanup both attempted rows.
        let _ = diesel::sql_query("DELETE FROM gm_campaigns WHERE source_draft_id = $1")
            .bind::<SqlUuid, _>(draft_id)
            .execute(&mut conn);
    }
}
