.DEFAULT_GOAL := help

# ─── Config (override via env) ────────────────────────────────────────────────
export APPLE_DEVELOPER_ID ?= Developer ID Application: Your Name (TEAMID)
export APPLE_TEAM_ID       ?= TEAMID
export NOTARY_PROFILE      ?= SpeakType

.PHONY: dev build clean install notary-setup server docker-build docker-up docker-down help

# ─── App (apps/widget-rust) ─────────────────────────────────────────────────────

dev: ## Run Tauri app in dev mode
	cd apps/widget-rust && cargo tauri dev

build: ## Clean → sidecar → app → sign → DMG → notarize, all in one (output: dist/)
	./scripts/log_manager.sh ./scripts/build.sh

# ─── Server (packages/server) ───────────────────────────────────────────────────

server: ## Run whisper server locally (Python)
	.venv/bin/python packages/server/whisper_server.py --model base

docker-build: ## Build server Docker image
	docker compose -f packages/server/docker-compose.yml build

docker-up: ## Start server in Docker
	docker compose -f packages/server/docker-compose.yml up -d

docker-down: ## Stop server Docker container
	docker compose -f packages/server/docker-compose.yml down

# ─── Setup & Utility ──────────────────────────────────────────────────────────

install: ## Create venv and install Python dependencies
	python3 -m venv .venv
	.venv/bin/pip install -e "packages/cli[all]"

clean: ## Remove build artifacts
	rm -rf dist logs
	rm -rf apps/widget-rust/src-tauri/target/release/bundle

notary-setup: ## Store notarization credentials in Keychain (one-time setup)
	@echo "→ This will store your Apple ID and app-specific password in the Keychain."
	@read -p "Apple ID email: " apple_id; \
	read -s -p "App-specific password: " apple_pwd; \
	echo ""; \
	xcrun notarytool store-credentials "$(NOTARY_PROFILE)" \
		--apple-id "$$apple_id" \
		--password "$$apple_pwd" \
		--team-id "$(APPLE_TEAM_ID)"
	@echo "→ Credentials stored as Keychain profile '$(NOTARY_PROFILE)'"

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
