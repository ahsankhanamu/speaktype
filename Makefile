.DEFAULT_GOAL := help

include make/config.mk

.PHONY: dev dev-fast build clean install install-python setup notary-setup server assets help

# ─── App (apps/widget) ───────────────────────────────────────────────────────

dev: ## Run app in dev mode (macOS: real .app bundle so Accessibility works)
	@if [ "$$(uname -s)" = "Darwin" ]; then \
		./tools/dev/dev-macos.sh; \
	else \
		cd apps/widget && cargo tauri dev; \
	fi

dev-fast: ## Run Tauri watch mode (fast rebuilds; Accessibility will NOT work on macOS)
	cd apps/widget && cargo tauri dev

build: ## Clean → sidecar → app → sign → DMG → notarize, all in one (output: dist/)
	./tools/build/log_manager.sh ./tools/build/build.sh

# ─── Server (packages/python/server) ─────────────────────────────────────────

server: ## Run whisper server locally (Python)
	.venv/bin/python packages/python/server/whisper_server.py --model base

assets: ## Regenerate README showcase images and demo animation
	.venv/bin/python tools/assets/compose_showcase.py

# ─── Setup & Utility ──────────────────────────────────────────────────────────

install: ## Set up dev environment (Python + macOS widget prerequisites + whisper.cpp)
	./tools/build/setup-dev.sh

install-python: ## Create venv and install Python dependencies only (requires Python 3.10+)
	@PY=""; \
	for candidate in python3.12 python3.11 python3; do \
		if command -v $$candidate >/dev/null 2>&1 && \
		   $$candidate -c 'import sys; sys.exit(0 if sys.version_info >= (3, 10) else 1)' 2>/dev/null; then \
			PY=$$candidate; \
			break; \
		fi; \
	done; \
	if [ -z "$$PY" ]; then \
		echo "Error: Python 3.10+ required. Install python3.12, python3.11, or upgrade python3."; \
		exit 1; \
	fi; \
	echo "Using $$($$PY --version) ($$PY)"; \
	$$PY -m venv .venv; \
	.venv/bin/pip install -e ".[all]"

setup: install ## Alias for install

clean: ## Remove build artifacts
	rm -rf dist logs
	rm -rf apps/widget/src-tauri/target/release/bundle

notary-setup: ## Store notarization credentials in Keychain (one-time setup)
	@echo "→ This will store your Apple ID and app-specific password in the Keychain."
	@apple_id="$(APPLE_ID_EMAIL)"; \
	if [ -z "$$apple_id" ]; then read -p "Apple ID email: " apple_id; fi; \
	if [ -n "$(NOTARY_PASSWORD)" ]; then \
		apple_pwd="$(NOTARY_PASSWORD)"; \
		export NOTARY_PASSWORD=""; \
	else \
		read -s -p "App-specific password: " apple_pwd; \
		echo ""; \
	fi; \
	xcrun notarytool store-credentials "$(NOTARY_PROFILE)" \
		--apple-id "$$apple_id" \
		--password "$$apple_pwd" \
		--team-id "$(APPLE_TEAM_ID)"
	@echo "→ Credentials stored as Keychain profile '$(NOTARY_PROFILE)'"

help: ## Show this help
	@grep -hE '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
