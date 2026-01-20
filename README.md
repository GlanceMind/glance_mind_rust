# GlanceMind

GlanceMind Rust monorepo containing the API service and database management.

## Project Structure

```
glance_mind_rust/
├── Cargo.toml              # Workspace configuration
├── Dockerfile.api          # API service Docker image
├── crates/
│   ├── api/                # GlanceMind API Service
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   ├── tests/
│   │   ├── Dockerfile      # (legacy, use root Dockerfile.api)
│   │   └── docker-compose.prod.yml
│   └── db/                 # Database schema and entities
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   ├── schema.rs   # Diesel auto-generated schema
│       │   └── entity/     # Entity definitions
│       └── migrations/
└── .github/
    └── workflows/
        ├── ci.yml          # CI for all crates
        └── deploy-api.yml  # API deployment
```

## Crates

### `glance_mind_db`

Centralized database schema and entity management.

- Diesel ORM schema definitions
- Entity structs (Queryable, Insertable, etc.)
- Database migrations
- Connection utilities

### `glance_mind_api`

REST API service for GlanceMind.

- Axum web framework
- JWT authentication
- Campaign management
- Social media integrations

## Development

### Prerequisites

- Rust (latest stable)
- PostgreSQL 15+
- Docker (for deployment)

### Setup

```bash
# Clone the repository
git clone <repo-url>
cd glance_mind_rust

# Copy environment file
cp .env.example .env
# Edit .env with your database credentials

# Run database migrations
cd crates/db
diesel migration run

# Build the workspace
cd ../..
cargo build

# Run tests
cargo test --workspace

# Run the API server
cargo run -p glance_mind_api
```

### Adding New Migrations

```bash
cd crates/db
diesel migration generate <migration_name>
# Edit the generated up.sql and down.sql
diesel migration run
```

## Deployment

### Manual Deployment

```bash
cd crates/api
docker-compose -f docker-compose.prod.yml up -d --build
```

### CI/CD

- **CI**: Runs on every push to `main` and PRs
- **Deploy**: Auto-triggers on push to `release*` branches/tags

## Dependencies

All workspace crates share common dependencies defined in the root `Cargo.toml`:

- `diesel` - Database ORM
- `axum` - Web framework
- `tokio` - Async runtime
- `serde` - Serialization
- `chrono` - Date/time handling

See `Cargo.toml` for the complete list.
