.DEFAULT_GOAL := help

include make/config.mk

.PHONY: dev dev-fast build clean install notary-setup server assets help

# ─── App (apps/widget-rust) ─────────────────────────────────────────────────────

dev: ## Run app in dev mode (macOS: real .app bundle so Accessibility works)
	@if [ "$$(uname -s)" = "Darwin" ]; then \
		./scripts/dev-macos.sh; \
	else \
		cd apps/widget-rust && cargo tauri dev; \
	fi

dev-fast: ## Run Tauri watch mode (fast rebuilds; Accessibility will NOT work on macOS)
	cd apps/widget-rust && cargo tauri dev

build: ## Clean → sidecar → app → sign → DMG → notarize, all in one (output: dist/)
	./scripts/log_manager.sh ./scripts/build.sh

# ─── Server (packages/server) ───────────────────────────────────────────────────

server: ## Run whisper server locally (Python)
	.venv/bin/python packages/server/whisper_server.py --model base

assets: ## Regenerate README showcase images and demo animation
	.venv/bin/python scripts/assets/compose_showcase.py

# ─── Setup & Utility ──────────────────────────────────────────────────────────

install: ## Create venv and install Python dependencies (requires Python 3.10+)
	@PY=""; \
	for candidate in python3.12 python3.11 python3; do \
		if command -v $$candidate >/dev/null 2>&1 && \
		   $$candidate -c 'import sys; sys.exit(0 if sys.version_info >= (3, 10) else 1)' 2>/dev/null; then \
			PY=$$candidate; \
			break; \
		fi; \
	done; \
	if [ -z "$$PY" ]; then \
		echo "Error: Python 3.10+ is required. Install python3.12, python3.11, or upgrade python3."; \
		exit 1; \
	fi; \
	echo "Using $$($$PY --version) ($$PY)"; \
	$$PY -m venv .venv; \
	.venv/bin/pip install -e ".[all]"

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
	@grep -hE '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
