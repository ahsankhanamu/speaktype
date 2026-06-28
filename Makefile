.DEFAULT_GOAL := help

include make/config.mk

.PHONY: dev build clean install notary-setup server assets help

# ─── App (apps/widget-rust) ─────────────────────────────────────────────────────

dev: ## Run Tauri app in dev mode
	cd apps/widget-rust && cargo tauri dev

build: ## Clean → sidecar → app → sign → DMG → notarize, all in one (output: dist/)
	./scripts/log_manager.sh ./scripts/build.sh

# ─── Server (packages/server) ───────────────────────────────────────────────────

server: ## Run whisper server locally (Python)
	.venv/bin/python packages/server/whisper_server.py --model base

assets: ## Regenerate README showcase images and demo animation
	python3 scripts/assets/compose_showcase.py

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
