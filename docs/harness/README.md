# Rust API and DB Harness

`glance_mind_rust` owns API routes, DTOs, state, and DB schema/entity behavior.
Harness changes should prefer API/DB contract tests and schema checks in v1.

Use:

- `cargo test --workspace`
- DB schema/entity/accounting focused tests for wallet and billing changes.

Do not promise a complete in-process API harness until route startup side
effects and background loops have explicit injection boundaries.
