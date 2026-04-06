# GlanceMind Rust API

<p align="center">
  Rust workspace for the GlanceMind backend API and database layer, providing the primary
  REST service, business logic, Diesel schema management, and deployment entrypoints.
</p>

<p align="center">
  <a href="https://github.com/GlanceMind/glance_mind_rust/actions/workflows/ci.yml">
    <img alt="Rust CI" src="https://github.com/GlanceMind/glance_mind_rust/actions/workflows/ci.yml/badge.svg?branch=main" />
  </a>
  <a href="https://github.com/GlanceMind/glance_mind_rust/actions/workflows/deploy-api.yml">
    <img alt="Deploy API" src="https://github.com/GlanceMind/glance_mind_rust/actions/workflows/deploy-api.yml/badge.svg?branch=main" />
  </a>
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2021-000000?logo=rust&logoColor=white" />
  <img alt="Axum" src="https://img.shields.io/badge/Axum-API-5E5CE6" />
  <img alt="Diesel" src="https://img.shields.io/badge/Diesel-PostgreSQL-1F6FEB" />
  <img alt="Tokio" src="https://img.shields.io/badge/Tokio-Async-111827" />
  <img alt="PostgreSQL" src="https://img.shields.io/badge/PostgreSQL-15+-4169E1?logo=postgresql&logoColor=white" />
  <img alt="Main Branch" src="https://img.shields.io/badge/Branch-main-111827" />
</p>

## Overview

`glance_mind_rust` is the core backend workspace for GlanceMind. It hosts:

- the main Axum-based API service
- shared DTOs, handlers, routes, and service logic
- database entities, migrations, and Diesel schema management
- deployment configuration for the backend API

This repository is the main backend entrypoint for frontend consumers, workflow orchestration, and database-backed business operations.

## Workspace Structure

```text
glance_mind_rust/
├── Cargo.toml              # Workspace definition
├── Cargo.lock
├── Dockerfile.api          # Primary API image build
├── crates/
│   ├── api/                # Axum API service
│   │   ├── src/
│   │   │   ├── dto/
│   │   │   ├── handler/
│   │   │   ├── routes/
│   │   │   ├── service/
│   │   │   ├── repository/
│   │   │   └── middleware/
│   │   └── tests/
│   └── db/                 # Diesel entities, schema, migrations
│       ├── migrations/
│       └── src/
│           ├── entity/
│           └── schema.rs
└── .github/workflows/      # CI and deployment workflows
```

## Crates

### `glance_mind_api`

The main backend service for the platform.

- Axum-based REST API
- JWT authentication and middleware
- Feature modules for campaigns, AI, drama, novel, wallet, materials, and more
- Service and repository layers for business logic orchestration

### `glance_mind_db`

Shared database layer and schema source of truth.

- Diesel schema definitions
- entity structs and model mappings
- migrations for schema evolution
- database utilities used by the API workspace

## Tech Stack

- `Rust 2021`
- `Axum`
- `Tokio`
- `Diesel`
- `PostgreSQL`
- `Serde`
- `Tracing`
- `Docker`

## Quick Start

### Prerequisites

- `Rust` stable toolchain
- `PostgreSQL 15+`
- `Diesel CLI` if you run migrations locally
- Docker, if you use containerized development or deployment

### Setup

```bash
git clone git@github.com:GlanceMind/glance_mind_rust.git
cd glance_mind_rust

# Configure environment as needed
cp .env.example .env

# Build the workspace
cargo build

# Run the API
cargo run -p glance_mind_api
```

## Common Commands

### Build and Check

```bash
cargo build
cargo check
```

### Test

```bash
cargo test --workspace
```

### Run the API

```bash
cargo run -p glance_mind_api
```

### Database Migrations

```bash
cd crates/db
diesel migration generate <migration_name>
diesel migration run
```

## Development Notes

- `crates/api/src/routes/` is the fastest way to locate API exposure
- `crates/api/src/service/` contains core business logic
- `crates/db/src/schema.rs` is generated schema output and should stay aligned with migrations
- database structure changes should go through migrations, not manual production-only SQL edits

## CI and Deployment

GitHub Actions is the primary automation layer for this repository.

- `Rust CI` tracks validation status on `main`
- `Deploy API` exposes deployment workflow status for `main`

Manual deployment entrypoints still exist for environment-specific operations:

```bash
cd crates/api
docker-compose -f docker-compose.prod.yml up -d --build
```

## Branch Strategy

- `main` is the primary development branch
- README badges track `main`
- release-line work should be merged back into `main` instead of continuing on a long-lived parallel branch

## Key Dependencies

Workspace-level dependencies include:

- `diesel`
- `diesel_migrations`
- `axum`
- `tokio`
- `serde`
- `serde_json`
- `chrono`
- `uuid`
- `tracing`

See `Cargo.toml` for the full shared dependency list and feature configuration.

## Related Repositories

- `glance_mind_front` for the primary web frontend
- `glance_mind_worker` for scheduler and execution flows
- `glance_mind_agent_rs` for agent-side integrations

Together, these repositories form the GlanceMind product and delivery stack.
