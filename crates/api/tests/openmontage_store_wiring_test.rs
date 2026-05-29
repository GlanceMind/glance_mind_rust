//! OpenMontage Job Store WIRING tests (B02)
//!
//! BUG B02 (P0): `crates/api/src/routes/root.rs` hardwires
//! `Arc::new(InMemoryJobStore::new())` for the OpenMontage service. The full
//! Diesel-backed `PgJobStore` (in `repository/openmontage_repository.rs`) is
//! never constructed outside its own tests, so in production every job/event
//! is lost on restart and the entire Pg store (completed->degraded rule,
//! ON CONFLICT, transactions) is dead code.
//!
//! SPEC encoded here:
//!   Production must use a Pg-backed `OpenMontageJobStore` when configured for
//!   prod (a DATABASE_URL is available / a store-mode selects postgres); jobs
//!   persist across a process restart. Dev/test may still use InMemory.
//!
//! ================= FIXER CONTRACT (the seam these tests reference) ==========
//! These tests are intentionally RED until the fixer implements the seam.
//! They are authored to FAIL TO COMPILE today (missing symbols) — a valid RED
//! for a missing-symbol contract — and to FAIL AN ASSERTION once the symbols
//! exist but `root.rs` still hardwires InMemory.
//!
//! The fixer MUST add, in `crate::repository::openmontage_repository`:
//!
//!   1. A trait method on `OpenMontageJobStore`:
//!          fn backend_name(&self) -> &'static str;
//!      with impls returning:
//!          InMemoryJobStore -> "memory"
//!          PgJobStore       -> "postgres"
//!
//!   2. A backend-selection config + constructor:
//!          pub struct OpenMontageStoreConfig {
//!              /// Some(url) when a Postgres DATABASE_URL is configured (prod).
//!              pub database_url: Option<String>,
//!              /// Optional explicit override: "postgres" | "memory".
//!              /// (e.g. read from an OPENMONTAGE_STORE / STORE_MODE env var)
//!              pub store_mode: Option<String>,
//!          }
//!
//!          pub fn build_openmontage_job_store(
//!              cfg: &OpenMontageStoreConfig,
//!          ) -> Arc<dyn OpenMontageJobStore>;
//!
//!      Selection rule the fixer must satisfy:
//!        - store_mode == Some("postgres")  -> Pg   (explicit prod override)
//!        - store_mode == Some("memory")    -> memory (explicit dev override)
//!        - else if database_url.is_some()  -> Pg   (prod default)
//!        - else                            -> memory (dev/test default)
//!
//!   3. `routes::root::routes` MUST construct the OpenMontage store via
//!      `build_openmontage_job_store(..)` (derived from the real env /
//!      `Database` config) instead of always `InMemoryJobStore::new()`.
//!
//! NOTE on DB-free determinism: `r2d2::Pool::builder().build()` is lazy and
//! does NOT open a TCP connection at build time, so the Pg branch can be
//! constructed in this test with a syntactically-valid-but-unconnected URL.
//! `backend_name()` must therefore not touch the database.
//! ============================================================================

#![cfg(test)]

use std::sync::Arc;

use glance_mind_api::repository::openmontage_repository::{
    build_openmontage_job_store, OpenMontageJobStore, OpenMontageStoreConfig,
};

/// A throwaway, non-routable Postgres URL. It is well-formed so the lazy r2d2
/// pool builds successfully, but no connection is attempted by `backend_name()`.
const FAKE_PROD_DATABASE_URL: &str = "postgres://aihub_user:aihub_password@127.0.0.1:1/aihub_db";

#[test]
fn prod_config_selects_postgres_backend() {
    // Prod-like: a DATABASE_URL is present and no explicit override.
    let cfg = OpenMontageStoreConfig {
        database_url: Some(FAKE_PROD_DATABASE_URL.to_string()),
        store_mode: None,
    };

    let store: Arc<dyn OpenMontageJobStore> = build_openmontage_job_store(&cfg);

    assert_eq!(
        store.backend_name(),
        "postgres",
        "prod config (DATABASE_URL present) must construct a Pg-backed store, \
         not the in-memory store that loses jobs on restart (B02)"
    );
}

#[test]
fn explicit_postgres_store_mode_selects_postgres_backend() {
    // Explicit prod override via store-mode env, DATABASE_URL also present.
    let cfg = OpenMontageStoreConfig {
        database_url: Some(FAKE_PROD_DATABASE_URL.to_string()),
        store_mode: Some("postgres".to_string()),
    };

    let store: Arc<dyn OpenMontageJobStore> = build_openmontage_job_store(&cfg);

    assert_eq!(
        store.backend_name(),
        "postgres",
        "store_mode=postgres must force the Pg-backed store (B02)"
    );
}

#[test]
fn dev_config_selects_memory_backend() {
    // Dev/test: no DATABASE_URL, no override -> in-memory is acceptable.
    let cfg = OpenMontageStoreConfig {
        database_url: None,
        store_mode: None,
    };

    let store: Arc<dyn OpenMontageJobStore> = build_openmontage_job_store(&cfg);

    assert_eq!(
        store.backend_name(),
        "memory",
        "dev config (no DATABASE_URL) may use the in-memory store"
    );
}

#[test]
fn explicit_memory_store_mode_overrides_present_database_url() {
    // Explicit dev override wins even when a DATABASE_URL happens to be set.
    let cfg = OpenMontageStoreConfig {
        database_url: Some(FAKE_PROD_DATABASE_URL.to_string()),
        store_mode: Some("memory".to_string()),
    };

    let store: Arc<dyn OpenMontageJobStore> = build_openmontage_job_store(&cfg);

    assert_eq!(
        store.backend_name(),
        "memory",
        "store_mode=memory must force the in-memory store even if DATABASE_URL is set"
    );
}
