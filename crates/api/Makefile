# GlanceMind API Makefile
# ========================
# Quick commands for E2E testing and development tasks

# Test environment configuration
DATABASE_URL := postgres://glancemind:testpassword@localhost:5434/glancemind_test
API_BASE_URL := http://localhost:8081
PAYMENT_NOTIFY_BASE_URL := http://host.docker.internal:8081
API_PORT := 8081
JWT_SECRET := e2e_test_jwt_secret_key_12345
XUNHUPAY_APP_ID := mock-app-id
XUNHUPAY_APP_SECRET := mock-xunhupay-secret
XUNHUPAY_GATEWAY_URL := http://localhost:8500
OPENAI_API_KEY := mock-openai-key
OPENAI_BASE_URL := http://localhost:8100/v1
LAOZHANG_API_KEY := mock-laozhang-key
LAOZHANG_BASE_URL := http://localhost:8100
TIKHUB_BASE_URL := http://localhost:8200
JIMENG_ACCESS_KEY_ID := mock-jimeng-ak
JIMENG_SECRET_ACCESS_KEY := mock-jimeng-sk
JIMENG_BASE_URL := http://localhost:8300
VIDU_API_KEY := mock-vidu-key
VIDU_BASE_URL := http://localhost:8400
TESTS_DIR := tests
DOCKER_COMPOSE := docker compose -f $(TESTS_DIR)/docker-compose.e2e.yml
API_PID_FILE := /tmp/glance_mind_api_e2e.pid

# Export environment variables
export DATABASE_URL
export API_BASE_URL

.PHONY: help build test-e2e test-rust up down logs clean setup-python start-api stop-api

# Default target
help:
	@echo "GlanceMind API - Available commands:"
	@echo ""
	@echo "  make build         - Build API project"
	@echo "  make test-e2e      - Run Python E2E tests (auto start/stop environment)"
	@echo "  make test-rust     - Run Rust integration tests"
	@echo "  make test-all      - Run all tests"
	@echo ""
	@echo "  make up            - Start test environment (PostgreSQL + API)"
	@echo "  make down          - Stop test environment"
	@echo "  make logs          - View test environment logs"
	@echo "  make clean         - Clean test environment"
	@echo ""
	@echo "  make setup-python  - Install Python test dependencies"
	@echo "  make db-shell      - Connect to test database"

# Build
build:
	cargo build --release

# Python E2E tests (full workflow)
test-e2e: setup-python build up-db-only wait-db start-api wait-api run-e2e-tests
	@$(MAKE) stop-api
	@$(MAKE) down
	@echo "\n✓ E2E tests completed"

# Run E2E tests only (assumes environment is already running)
test-e2e-only: setup-python
	@echo "\n>>> Running E2E tests..."
	cd $(TESTS_DIR) && python -m pytest -v --tb=short

# Run specific test files
test-comments: setup-python
	cd $(TESTS_DIR) && python -m pytest test_comments_api.py -v --tb=short

test-auth: setup-python
	cd $(TESTS_DIR) && python -m pytest test_auth_api.py -v --tb=short

test-config: setup-python
	cd $(TESTS_DIR) && python -m pytest test_config_api.py -v --tb=short

# Internal target: run E2E tests
run-e2e-tests:
	@echo "\n>>> Running E2E tests..."
	cd $(TESTS_DIR) && python -m pytest -v --tb=short; \
	status=$$?; \
	if [ $$status -ne 0 ]; then \
		$(MAKE) stop-api; \
		$(MAKE) down; \
		exit $$status; \
	fi

# Rust integration tests
test-rust:
	cargo test --test '*' -- --test-threads=1

# Full-pipeline E2E tests (API + Scheduler + Agent-RS)
test-pipeline:
	@echo "\n>>> Running full-pipeline E2E tests..."
	cd $(TESTS_DIR)/e2e && $(MAKE) run

test-pipeline-up:
	cd $(TESTS_DIR)/e2e && $(MAKE) up

test-pipeline-down:
	cd $(TESTS_DIR)/e2e && $(MAKE) down

# All tests
test-all: test-e2e test-rust

# Start API server in background
start-api:
	@echo ">>> Starting API server on port $(API_PORT)..."
	@pkill -f "glance_mind_api" 2>/dev/null || true
	@sleep 1
	@DATABASE_URL="$(DATABASE_URL)" \
		JWT_SECRET="$(JWT_SECRET)" \
		API_BASE_URL="$(PAYMENT_NOTIFY_BASE_URL)" \
		XUNHUPAY_APP_ID="$(XUNHUPAY_APP_ID)" \
		XUNHUPAY_APP_SECRET="$(XUNHUPAY_APP_SECRET)" \
		XUNHUPAY_GATEWAY_URL="$(XUNHUPAY_GATEWAY_URL)" \
		OPENAI_API_KEY="$(OPENAI_API_KEY)" \
		OPENAI_BASE_URL="$(OPENAI_BASE_URL)" \
		LAOZHANG_API_KEY="$(LAOZHANG_API_KEY)" \
		LAOZHANG_BASE_URL="$(LAOZHANG_BASE_URL)" \
		TIKHUB_BASE_URL="$(TIKHUB_BASE_URL)" \
		JIMENG_ACCESS_KEY_ID="$(JIMENG_ACCESS_KEY_ID)" \
		JIMENG_SECRET_ACCESS_KEY="$(JIMENG_SECRET_ACCESS_KEY)" \
		JIMENG_BASE_URL="$(JIMENG_BASE_URL)" \
		VIDU_API_KEY="$(VIDU_API_KEY)" \
		VIDU_BASE_URL="$(VIDU_BASE_URL)" \
		PORT="$(API_PORT)" \
		RUST_LOG=info \
		nohup ../../target/release/glance_mind_api > /tmp/glance_mind_api.log 2>&1 & \
		echo $$! > $(API_PID_FILE)
	@echo "API server started (PID: $$(cat $(API_PID_FILE)))"

# Stop API server
stop-api:
	@echo ">>> Stopping API server..."
	@if [ -f $(API_PID_FILE) ]; then \
		kill $$(cat $(API_PID_FILE)) 2>/dev/null || true; \
		rm -f $(API_PID_FILE); \
	fi
	@pkill -f "glance_mind_api" 2>/dev/null || true
	@echo "API server stopped"

# Start full test environment (DB + API)
up: up-db-only wait-db start-api wait-api
	@echo "✓ Test environment ready"
	@echo "Database: localhost:5434"
	@echo "API: localhost:$(API_PORT)"
	@echo "LaoZhang mock: localhost:8100"
	@echo "TikHub mock: localhost:8200"
	@echo "Jimeng mock: localhost:8300"
	@echo "Vidu mock: localhost:8400"
	@echo "XunhuPay mock: localhost:8500"

# Start database only (for local development)
up-db-only:
	@echo ">>> Starting test database and mock services..."
	$(DOCKER_COMPOSE) build db laozhang-mock tikhub-mock jimeng-mock vidu-mock xunhupay-mock
	$(DOCKER_COMPOSE) up -d db laozhang-mock tikhub-mock jimeng-mock vidu-mock xunhupay-mock
	@echo "Database: localhost:5434"

# Wait for services to be ready
wait-ready: wait-db wait-api
	@echo "✓ Test environment ready"

wait-db:
	@echo ">>> Waiting for database..."
	@sleep 5
	@until docker exec api-e2e-db pg_isready -U glancemind -d glancemind_test > /dev/null 2>&1; do \
		echo "Waiting for PostgreSQL..."; \
		sleep 2; \
	done
	@echo "✓ Database ready"

wait-api:
	@echo ">>> Waiting for API..."
	@for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15; do \
		if curl -sf http://localhost:$(API_PORT)/health > /dev/null 2>&1; then \
			echo "✓ API ready"; \
			exit 0; \
		fi; \
		echo "Waiting for API... ($$i/15)"; \
		sleep 2; \
	done; \
	echo "API startup timeout. Check /tmp/glance_mind_api.log for details"; \
	cat /tmp/glance_mind_api.log 2>/dev/null || true; \
	exit 1

# Stop test environment
down: stop-api
	@echo ">>> Stopping test environment..."
	$(DOCKER_COMPOSE) down -v 2>/dev/null || true

# View logs
logs:
	$(DOCKER_COMPOSE) logs -f

logs-db:
	$(DOCKER_COMPOSE) logs -f db

logs-api:
	$(DOCKER_COMPOSE) logs -f api

# Clean up
clean: down
	@echo ">>> Cleaning up..."
	rm -rf $(TESTS_DIR)/__pycache__
	rm -rf $(TESTS_DIR)/.pytest_cache
	cargo clean

# Install Python dependencies
setup-python:
	@echo ">>> Checking Python dependencies..."
	@pip install -q -r $(TESTS_DIR)/requirements.txt 2>/dev/null || pip3 install -q -r $(TESTS_DIR)/requirements.txt

# Connect to database
db-shell:
	docker exec -it api-e2e-db psql -U glancemind -d glancemind_test

# View database tables
db-tables:
	docker exec api-e2e-db psql -U glancemind -d glancemind_test -c "\dt gm_*"

# View test data statistics
db-stats:
	@docker exec api-e2e-db psql -U glancemind -d glancemind_test -c "\
		SELECT 'Platforms' as table_name, COUNT(*) as count FROM gm_platforms UNION ALL \
		SELECT 'Users', COUNT(*) FROM gm_users UNION ALL \
		SELECT 'Campaigns', COUNT(*) FROM gm_campaigns UNION ALL \
		SELECT 'TikTok Videos', COUNT(*) FROM gm_agent_videos UNION ALL \
		SELECT 'TikTok Comments', COUNT(*) FROM gm_agent_comments UNION ALL \
		SELECT 'Facebook Posts', COUNT(*) FROM gm_agent_facebook_posts UNION ALL \
		SELECT 'Facebook Comments', COUNT(*) FROM gm_agent_facebook_comments UNION ALL \
		SELECT 'Instagram Posts', COUNT(*) FROM gm_agent_instagram_posts UNION ALL \
		SELECT 'Instagram Comments', COUNT(*) FROM gm_agent_instagram_comments UNION ALL \
		SELECT 'Reddit Posts', COUNT(*) FROM gm_agent_reddit_posts UNION ALL \
		SELECT 'Reddit Comments', COUNT(*) FROM gm_agent_reddit_comments UNION ALL \
		SELECT 'Twitter Tweets', COUNT(*) FROM gm_agent_twitter_tweets UNION ALL \
		SELECT 'Twitter Comments', COUNT(*) FROM gm_agent_twitter_comments \
		ORDER BY table_name;"
