.PHONY: help dev dev-down dev-logs \
	infisical-check pull-secrets upload-secrets \
	cli dev-ui package-ui test-smoke

.DEFAULT_GOAL := help

COMPOSE ?= docker compose
COMPOSE_FILES ?= -f docker-compose.yml
SERVER_PORT ?= 3000
GOOSE_SERVER__SECRET_KEY ?= avcd-agent-local-development-key

INFISICAL_API_URL ?= https://secrets.avcd.ai/api
INFISICAL_PROJECT_ID ?=
INFISICAL_SECRET_PATH ?= /
INFISICAL_ENV ?= dev
INFISICAL_PUSH_FILE ?= .env.local
INFISICAL_PULL_FILE ?= .env.local
INFISICAL_CREDENTIALS_FILE ?= ../infisical/.env

help:
	@echo "AVCD Agent development"
	@echo "  make dev             Build and start the local ACP backend"
	@echo "  make dev-down        Stop the local development stack"
	@echo "  make dev-logs        Follow backend logs"
	@echo "  make cli             Open the AVCD Agent CLI in Docker"
	@echo "  make dev-ui          Run Electron on the host against Docker"
	@echo "  make package-ui      Build a local desktop package"
	@echo "  make test-smoke      Verify backend, CLI, branding, and package"
	@echo "  make pull-secrets    Export Infisical dev secrets to .env.local"
	@echo "  make upload-secrets  Upload .env.local to Infisical"

dev:
	SERVER_PORT="$(SERVER_PORT)" GOOSE_SERVER__SECRET_KEY="$(GOOSE_SERVER__SECRET_KEY)" \
		$(COMPOSE) $(COMPOSE_FILES) up -d --build

dev-down:
	$(COMPOSE) $(COMPOSE_FILES) down

dev-logs:
	$(COMPOSE) $(COMPOSE_FILES) logs -f server

# -----------------------------------------------------------------------------
# Infisical secrets (local synchronization only)

infisical-check:
	@command -v infisical >/dev/null 2>&1 || (echo "Infisical CLI not installed. Run: brew install infisical/get-cli/infisical" && exit 1)
	@test -n "$(INFISICAL_PROJECT_ID)" || (echo "Set INFISICAL_PROJECT_ID for the avcd-agent project" && exit 1)
	@test -f "$(INFISICAL_CREDENTIALS_FILE)" || test -n "$$INFISICAL_CLIENT_ID" || (echo "Missing Infisical credentials: $(INFISICAL_CREDENTIALS_FILE)" && exit 1)
	@echo "Infisical CLI and project configuration ready"

pull-secrets: infisical-check
	@set -a; [ -f "$(INFISICAL_CREDENTIALS_FILE)" ] && . "$(INFISICAL_CREDENTIALS_FILE)"; set +a; \
	INFISICAL_TOKEN=$$(infisical login --method=universal-auth \
		--client-id="$$INFISICAL_CLIENT_ID" --client-secret="$$INFISICAL_CLIENT_SECRET" \
		--domain="$(patsubst %/api,%,$(INFISICAL_API_URL))" --silent --plain); \
	infisical export --env="$(INFISICAL_ENV)" --path="$(INFISICAL_SECRET_PATH)" \
		--projectId="$(INFISICAL_PROJECT_ID)" --token="$$INFISICAL_TOKEN" \
		--format=dotenv --domain="$(patsubst %/api,%,$(INFISICAL_API_URL))" --silent \
		> "$(INFISICAL_PULL_FILE)"; \
	test -s "$(INFISICAL_PULL_FILE)" || (echo "Infisical export was empty" && exit 1); \
	echo "Exported secrets to $(INFISICAL_PULL_FILE)"

upload-secrets: infisical-check
	@test -s "$(INFISICAL_PUSH_FILE)" || (echo "Secret source is empty: $(INFISICAL_PUSH_FILE)" && exit 1)
	@set -a; [ -f "$(INFISICAL_CREDENTIALS_FILE)" ] && . "$(INFISICAL_CREDENTIALS_FILE)"; set +a; \
	INFISICAL_TOKEN=$$(infisical login --method=universal-auth \
		--client-id="$$INFISICAL_CLIENT_ID" --client-secret="$$INFISICAL_CLIENT_SECRET" \
		--domain="$(patsubst %/api,%,$(INFISICAL_API_URL))" --silent --plain); \
	infisical secrets set --file="$(INFISICAL_PUSH_FILE)" --env="$(INFISICAL_ENV)" \
		--path="$(INFISICAL_SECRET_PATH)" --projectId="$(INFISICAL_PROJECT_ID)" \
		--token="$$INFISICAL_TOKEN" --domain="$(patsubst %/api,%,$(INFISICAL_API_URL))" --silent; \
	echo "Uploaded $(INFISICAL_PUSH_FILE) to Infisical"

# -----------------------------------------------------------------------------
# Repository-specific developer targets

cli:
	$(COMPOSE) $(COMPOSE_FILES) --profile cli run --rm cli

dev-ui:
	cd ui/desktop && \
		GOOSE_EXTERNAL_BACKEND=true \
		GOOSE_SERVER__SECRET_KEY="$(GOOSE_SERVER__SECRET_KEY)" \
		pnpm install --frozen-lockfile && pnpm run start-gui

package-ui:
	cd ui/desktop && pnpm install --frozen-lockfile && pnpm run package

test-smoke:
	SERVER_PORT="$(SERVER_PORT)" ./scripts/smoke-test.sh
