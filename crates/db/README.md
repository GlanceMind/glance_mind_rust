# GlanceMind Database

Centralized database schema and migration management for GlanceMind microservices.

## Overview

This repository is the **single source of truth** for:

- Database schema definitions
- Database migrations
- Schema synchronization across services

## Migration Strategy

### Baseline Migration

Starting from 2026-01-20, we use a **Baseline Migration** strategy:

- All historical migrations are consolidated into a single `baseline` migration
- New migrations start from after the baseline
- The original 41 migrations are archived in `archive/migrations_legacy/`

### New Environment Deployment

```bash
# Just run migrations directly
diesel migration run
```

### Existing Environment (Production/Staging)

**Important**: Before running new migrations, you must mark the baseline:

```bash
# Using the script (recommended)
./scripts/migrate-existing-db.sh

# Or execute SQL directly
psql $DATABASE_URL -f scripts/migrate_existing_db.sql
```

This replaces old migration records with the baseline record without affecting business data.

## Project Structure

```
glance_mind_db/
├── migrations/                     # Database migrations
│   └── 2026-01-20-*_baseline/     # Baseline (complete schema)
├── archive/
│   └── migrations_legacy/          # 41 legacy migrations (for reference)
├── src/
│   ├── lib.rs                     # Library exports and utilities
│   ├── schema.rs                  # Diesel schema (auto-generated)
│   ├── main.rs                    # Help information
│   └── bin/
│       ├── migrate.rs             # Migration CLI tool
│       └── schema.rs              # Schema management tool
├── scripts/
│   ├── sync-schema.sh             # Sync schema to dependent services
│   ├── export-schema.sh           # Export schema from database
│   ├── generate-migration.sh      # Generate new migration
│   ├── migrate-existing-db.sh     # Migrate existing database
│   └── migrate_existing_db.sql    # Migration SQL script
├── .github/workflows/
│   ├── migration.yml              # CI/CD for migrations
│   └── schema-sync.yml            # Schema sync validation
├── diesel.toml                    # Diesel configuration
└── Cargo.toml
```

## Quick Start

### Prerequisites

- Rust toolchain (1.70+)
- PostgreSQL (15+)
- Diesel CLI: `cargo install diesel_cli --no-default-features --features postgres`

### Setup

1. Clone the repository
2. Copy `.env.example` to `.env` and configure `DATABASE_URL`
3. Run migrations:

```bash
diesel migration run
```

### Common Commands

```bash
# Run pending migrations
diesel migration run

# Revert last migration
diesel migration revert

# Check migration status
cargo run --bin db-migrate -- --status

# Generate new migration
./scripts/generate-migration.sh add_new_feature

# Export schema from database
./scripts/export-schema.sh

# Sync schema to dependent services
./scripts/sync-schema.sh

# Export and sync in one command
./scripts/export-schema.sh --sync
```

## Using in Dependent Services

### As a Cargo Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
glance_mind_db = { path = "../glance_mind_db" }
```

Then use in your code:

```rust
use glance_mind_db::schema::*;
use glance_mind_db::{establish_connection, run_migrations};

fn main() {
    // Get a database connection
    let mut conn = establish_connection();
    
    // Run migrations at startup (optional)
    run_migrations(&mut conn).expect("Failed to run migrations");
    
    // Use schema tables
    use glance_mind_db::schema::gm_users::dsl::*;
    // ...
}
```

### Schema Sync

The `schema.rs` file should be copied to dependent services:

```bash
# Run from glance_mind_db directory
./scripts/sync-schema.sh
```

Target services:

- `glance_mind_api/src/schema.rs`
- `glance_mind_worker/glance_mind_scheduler/src/schema.rs`

## Development Workflow

### Creating a New Migration

1. Generate migration files:

   ```bash
   ./scripts/generate-migration.sh add_user_preferences
   ```

2. Edit the generated files:
   - `migrations/<timestamp>_add_user_preferences/up.sql`
   - `migrations/<timestamp>_add_user_preferences/down.sql`

3. Run the migration:

   ```bash
   diesel migration run
   ```

4. Update and sync schema:

   ```bash
   ./scripts/export-schema.sh --sync
   ```

5. Commit changes:

   ```bash
   git add migrations/ src/schema.rs
   git commit -m "feat: add user preferences table"
   ```

### Testing Migrations

```bash
# Run all migrations on test database
DATABASE_URL=postgres://test:test@localhost/test_db diesel migration run

# Revert and re-run to test up/down
diesel migration revert
diesel migration run
```

## CI/CD

### Workflow: Migration Deployment

Triggers:

- Push to `main` branch → Deploy to staging
- Push to `release/*` branch → Deploy to production
- Manual trigger → Choose environment

### Workflow: Schema Sync Check

On pull requests:

- Validates migrations can be applied
- Checks schema.rs matches migration output
- Build and test checks

### Required Secrets

Configure in GitHub repository settings:

- `STAGING_DATABASE_URL` - Staging database connection string
- `PRODUCTION_DATABASE_URL` - Production database connection string

## Best Practices

1. **Always test migrations locally** before pushing
2. **Include down.sql** for every migration (for rollback capability)
3. **Keep migrations small** and focused on single changes
4. **Never modify existing migrations** that have been deployed
5. **Sync schema.rs** after every migration
6. **Use transactions** in migrations when possible

## Troubleshooting

### Migration fails with "relation already exists"

The migration may have been partially applied. Check the `__diesel_schema_migrations` table:

```sql
SELECT * FROM __diesel_schema_migrations ORDER BY run_on DESC;
```

### Schema.rs is out of sync

Regenerate from database:

```bash
./scripts/export-schema.sh
```

### Connection refused

Check:

1. PostgreSQL is running
2. `DATABASE_URL` is correct in `.env`
3. User has proper permissions

## License

Proprietary - GlanceMind Team
