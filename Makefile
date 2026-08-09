.PHONY: help dev dev-down dev-logs \
	infisical-check pull-secrets upload-secrets \
	ensure-local-env cli dev-ui build-desktop-binary package-ui test-smoke validate-openrouter \
	check-core clippy sync-i18n check-ui \
	gateway-install gateway-test gateway-test-e2e gateway-dev gateway-up gateway-down

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
	@echo "Avocado Work development"
	@echo "  make dev             Build and start the local ACP backend"
	@echo "  make dev-down        Stop the local development stack"
	@echo "  make dev-logs        Follow backend logs"
	@echo "  make cli             Open the Avocado Work CLI in Docker"
	@echo "  make dev-ui          Run Electron on the host against Docker (Node 24+)"
	@echo "  make package-ui      Build a local desktop package"
	@echo "  make test-smoke      Verify backend, CLI, branding, and package"
	@echo "  make validate-openrouter  Verify the AVCD OpenRouter provider preset"
	@echo "  make check-core      Type-check the Rust core and CLI"
	@echo "  make clippy          Run strict Rust workspace linting"
	@echo "  make sync-i18n       Refresh desktop English messages"
	@echo "  make check-ui        Type-check, lint, and test the desktop"
	@echo "  make pull-secrets    Export Infisical dev secrets to .env.local"
	@echo "  make upload-secrets  Upload .env.local to Infisical"
	@echo "  make gateway-test    Unit-test avcd-agent-gateway"
	@echo "  make gateway-test-e2e E2E-test avcd-agent-gateway isolation"
	@echo "  make gateway-up      Build/start gateway via its compose file"
	@echo "  make gateway-down    Stop gateway compose stack"

ensure-local-env:
	@test -f .env.local || (cp .env.local.example .env.local && echo "Created .env.local from .env.local.example")

dev: ensure-local-env
	SERVER_PORT="$(SERVER_PORT)" GOOSE_SERVER__SECRET_KEY="$(GOOSE_SERVER__SECRET_KEY)" \
		$(COMPOSE) $(COMPOSE_FILES) up -d --build

dev-down: ensure-local-env
	$(COMPOSE) $(COMPOSE_FILES) down

dev-logs: ensure-local-env
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

WITH_NODE := ./scripts/with-node.sh

cli: ensure-local-env
	$(COMPOSE) $(COMPOSE_FILES) --profile cli run --rm cli

dev-ui:
	@SERVER_PORT="$(SERVER_PORT)" GOOSE_SERVER__SECRET_KEY="$(GOOSE_SERVER__SECRET_KEY)" ./scripts/prepare-dev-ui-env.sh
	@curl -sf -H "X-Secret-Key: $(GOOSE_SERVER__SECRET_KEY)" "http://127.0.0.1:$(SERVER_PORT)/status" >/dev/null \
		|| (echo "ACP backend is not running. Start it first: make dev" && exit 1)
	@echo "Desktop UI: $$(./scripts/with-node.sh 24 bash -c 'echo node $$(node -v), pnpm $$(pnpm -v)')"
	$(WITH_NODE) 24 bash -c 'cd ui/desktop && \
		set -a && . ./.env && set +a && \
		DOTENV_CONFIG_PATH="$$(pwd)/.env" \
		pnpm install --frozen-lockfile && pnpm run start-gui'

build-desktop-binary:
	cargo build -p goose-cli --bin goose --no-default-features \
		--features rustls-tls,tui,disable-update
	cp target/debug/goose ui/desktop/src/bin/goose

package-ui: build-desktop-binary
	$(WITH_NODE) 22 bash -c 'cd ui/desktop && pnpm install --frozen-lockfile && pnpm run package && \
		(test -d "out/Avocado Work-darwin-arm64/Avocado Work.app" || \
			pnpm exec electron-packager . "Avocado Work" \
				--platform=darwin --arch=arm64 --out=out --overwrite --asar \
				--executable-name=avocado-work --icon=src/images/icon.icns \
				--extra-resource=src/bin --extra-resource=src/images \
				--extra-resource=src/app-update.yml) && \
		test -d "out/Avocado Work-darwin-arm64/Avocado Work.app"'

test-smoke:
	SERVER_PORT="$(SERVER_PORT)" ./scripts/smoke-test.sh

validate-openrouter: ensure-local-env
	./scripts/validate-openrouter-preset.sh all

check-core:
	cargo check -p goose -p goose-cli --no-default-features \
		--features rustls-tls,tui,disable-update

clippy:
	docker build --file Dockerfile.dev --target lint .

sync-i18n:
	cd ui/desktop && pnpm install --frozen-lockfile && pnpm run i18n:extract

check-ui:
	$(WITH_NODE) 24 bash -c 'cd ui/desktop && pnpm install --frozen-lockfile && \
		pnpm run lint:check && pnpm run test:run'

gateway-install:
	cd services/avcd-agent-gateway && npm ci

gateway-test: gateway-install
	cd services/avcd-agent-gateway && npm run test:unit

gateway-test-e2e: gateway-install
	cd services/avcd-agent-gateway && npm run test:e2e

gateway-dev:
	cd services/avcd-agent-gateway && AVCD_GATEWAY_MAIN=1 npm run dev

gateway-up:
	$(COMPOSE) -f services/avcd-agent-gateway/docker-compose.yml up -d --build

gateway-down:
	$(COMPOSE) -f services/avcd-agent-gateway/docker-compose.yml down
