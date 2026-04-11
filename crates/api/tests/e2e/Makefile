.PHONY: up test run down logs clean

COMPOSE := docker compose -f docker-compose.full-pipeline.yml

up:
	$(COMPOSE) up -d --build
	@echo "Waiting for services to be healthy..."
	@sleep 10
	@echo "Full pipeline environment ready."

test:
	$(COMPOSE) exec test-runner pytest -v --tb=short -x

test-aipub:
	$(COMPOSE) exec test-runner pytest -v --tb=short -x -m aipub

test-campaign:
	$(COMPOSE) exec test-runner pytest -v --tb=short -x -m campaign

run: up test

down:
	$(COMPOSE) down -v --remove-orphans

logs:
	$(COMPOSE) logs -f

clean: down
	$(COMPOSE) down --rmi local -v --remove-orphans
