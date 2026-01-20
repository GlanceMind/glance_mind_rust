# GlanceMind Rust Workspace Makefile
# ===================================
# Quick commands for development workflow

.PHONY: help db-up db-down db-restart db-logs db-shell \
        migrate migrate-new migrate-status schema-sync schema-export \
        build test check clippy fmt clean \
        dev dev-down dev-restart dev-logs \
        api-up api-down api-logs \
        setup all

# Default target
.DEFAULT_GOAL := help

# Container names
DB_CONTAINER := glance-mind-db
API_CONTAINER := glance-mind-api
DATABASE_URL ?= postgres://aihub_user:aihub_password@localhost:5432/aihub_db

# Colors
CYAN := \033[36m
GREEN := \033[32m
YELLOW := \033[33m
RESET := \033[0m

# ============================================================================
# Help
# ============================================================================

help: ## Show this help message
	@echo "$(CYAN)GlanceMind Rust Workspace$(RESET)"
	@echo "========================="
	@echo ""
	@echo "$(GREEN)Database Commands:$(RESET)"
	@grep -E '^db-[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(GREEN)Migration Commands:$(RESET)"
	@grep -E '^migrate[a-zA-Z_-]*:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'
	@grep -E '^schema[a-zA-Z_-]*:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(GREEN)Rust Commands:$(RESET)"
	@grep -E '^(build|test|check|clippy|fmt|clean):.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(GREEN)API Commands:$(RESET)"
	@grep -E '^api-[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(GREEN)Development Commands:$(RESET)"
	@grep -E '^(dev|dev-[a-zA-Z_-]+|setup|all):.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'

# ============================================================================
# Database Commands
# ============================================================================

db-up: ## Start PostgreSQL database container
	@echo "$(GREEN)Starting database...$(RESET)"
	cd crates/api && docker-compose up -d db
	@echo "$(GREEN)Waiting for database to be healthy...$(RESET)"
	@until docker exec $(DB_CONTAINER) pg_isready -U aihub_user -d aihub_db > /dev/null 2>&1; do \
		sleep 1; \
	done
	@echo "$(GREEN)Database is ready!$(RESET)"

db-down: ## Stop PostgreSQL database container
	@echo "$(YELLOW)Stopping database...$(RESET)"
	cd crates/api && docker-compose down

db-restart: ## Restart PostgreSQL database container
	@echo "$(YELLOW)Restarting database...$(RESET)"
	$(MAKE) db-down
	$(MAKE) db-up

db-logs: ## Show database container logs
	docker logs -f $(DB_CONTAINER)

db-shell: ## Open psql shell in database container
	docker exec -it $(DB_CONTAINER) psql -U aihub_user -d aihub_db

db-reset: ## Reset database (WARNING: deletes all data)
	@echo "$(YELLOW)WARNING: This will delete all database data!$(RESET)"
	@read -p "Are you sure? (y/N): " confirm && [ "$$confirm" = "y" ] || exit 1
	cd crates/api && docker-compose down -v
	$(MAKE) db-up
	$(MAKE) migrate

# ============================================================================
# Migration Commands
# ============================================================================

migrate: ## Run pending database migrations
	@echo "$(GREEN)Running migrations...$(RESET)"
	cd crates/db && DATABASE_URL=$(DATABASE_URL) diesel migration run
	@echo "$(GREEN)Migrations completed!$(RESET)"

migrate-new: ## Create a new migration (usage: make migrate-new NAME=add_users_table)
	@if [ -z "$(NAME)" ]; then \
		echo "$(YELLOW)Usage: make migrate-new NAME=migration_name$(RESET)"; \
		exit 1; \
	fi
	@echo "$(GREEN)Creating migration: $(NAME)$(RESET)"
	cd crates/db && diesel migration generate $(NAME)

migrate-status: ## Show migration status
	cd crates/db && DATABASE_URL=$(DATABASE_URL) diesel migration list

migrate-redo: ## Redo the last migration (rollback + run)
	@echo "$(YELLOW)Redoing last migration...$(RESET)"
	cd crates/db && DATABASE_URL=$(DATABASE_URL) diesel migration redo

migrate-revert: ## Revert the last migration
	@echo "$(YELLOW)Reverting last migration...$(RESET)"
	cd crates/db && DATABASE_URL=$(DATABASE_URL) diesel migration revert

schema-sync: ## Sync schema.rs from database
	@echo "$(GREEN)Syncing schema...$(RESET)"
	cd crates/db && ./scripts/sync-schema.sh

schema-export: ## Export database schema to SQL file
	@echo "$(GREEN)Exporting schema...$(RESET)"
	cd crates/db && ./scripts/export-schema.sh

# ============================================================================
# Rust Commands
# ============================================================================

build: ## Build all crates
	@echo "$(GREEN)Building workspace...$(RESET)"
	cargo build --workspace

build-release: ## Build all crates in release mode
	@echo "$(GREEN)Building workspace (release)...$(RESET)"
	cargo build --workspace --release

test: ## Run all tests
	@echo "$(GREEN)Running tests...$(RESET)"
	cargo test --workspace

test-verbose: ## Run all tests with verbose output
	cargo test --workspace -- --nocapture

check: ## Check code without building
	cargo check --workspace --all-targets

clippy: ## Run clippy linter
	@echo "$(GREEN)Running clippy...$(RESET)"
	cargo clippy --workspace --all-targets -- -D warnings

fmt: ## Format code
	@echo "$(GREEN)Formatting code...$(RESET)"
	cargo fmt --all

fmt-check: ## Check code formatting
	cargo fmt --all -- --check

clean: ## Clean build artifacts
	@echo "$(YELLOW)Cleaning build artifacts...$(RESET)"
	cargo clean

# ============================================================================
# API Commands
# ============================================================================

api-up: ## Start API service with docker-compose
	@echo "$(GREEN)Starting API service...$(RESET)"
	cd crates/api && docker-compose up -d

api-down: ## Stop API service
	@echo "$(YELLOW)Stopping API service...$(RESET)"
	cd crates/api && docker-compose down

api-restart: ## Restart API service
	$(MAKE) api-down
	$(MAKE) api-up

api-logs: ## Show API service logs
	docker logs -f $(API_CONTAINER)

api-build: ## Build API docker image
	@echo "$(GREEN)Building API docker image...$(RESET)"
	docker build -t glance-mind-api:latest -f Dockerfile.api .

api-run: ## Run API locally (without docker)
	@echo "$(GREEN)Starting API locally...$(RESET)"
	cd crates/api && DATABASE_URL=$(DATABASE_URL) cargo run

# ============================================================================
# Development Commands
# ============================================================================

dev: ## Start full development environment (database + backend)
	@echo "$(GREEN)Starting development environment...$(RESET)"
	cd crates/api && docker-compose up -d
	@echo "$(GREEN)Development environment started!$(RESET)"
	@echo ""
	@echo "Services:"
	@echo "  - Database: localhost:5432"
	@echo "  - API:      http://localhost:8000"

dev-down: ## Stop development environment
	@echo "$(YELLOW)Stopping development environment...$(RESET)"
	cd crates/api && docker-compose down

dev-restart: ## Rebuild and restart backend service
	@echo "$(YELLOW)Rebuilding and restarting backend...$(RESET)"
	cd crates/api && docker-compose up -d --build backend

dev-logs: ## Show all service logs
	cd crates/api && docker-compose logs -f

dev-db-only: db-up migrate ## Start only database (for local API development)
	@echo "$(GREEN)Database ready!$(RESET)"
	@echo ""
	@echo "Database: $(DATABASE_URL)"
	@echo ""
	@echo "Run API locally: make api-run"

setup: ## Initial project setup
	@echo "$(GREEN)Setting up project...$(RESET)"
	@echo "Installing diesel_cli..."
	cargo install diesel_cli --no-default-features --features postgres
	@echo ""
	@echo "$(GREEN)Setup complete!$(RESET)"
	@echo "Run 'make dev' to start development environment"

all: fmt clippy test ## Run format, lint, and tests
	@echo "$(GREEN)All checks passed!$(RESET)"

ci: fmt-check clippy test ## CI pipeline (format check + lint + tests)
	@echo "$(GREEN)CI checks passed!$(RESET)"
