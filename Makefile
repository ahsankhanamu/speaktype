.DEFAULT_GOAL := help

# ─── Widget (speaktype-rust/) ────────────────────────────────────────────────

.PHONY: dev build

dev: ## Run Tauri app in dev mode
	cd speaktype-rust && cargo tauri dev

build: ## Build Tauri app (.dmg on macOS)
	cd speaktype-rust && cargo tauri build

# ─── Server (server/) ────────────────────────────────────────────────────────

.PHONY: server docker-build docker-up docker-down

server: ## Run whisper server locally (Python)
	.venv/bin/python server/whisper_server.py --model base

docker-build: ## Build server Docker image
	docker compose -f server/docker-compose.yml build

docker-up: ## Start server in Docker
	docker compose -f server/docker-compose.yml up -d

docker-down: ## Stop server Docker container
	docker compose -f server/docker-compose.yml down

# ─── General ──────────────────────────────────────────────────────────────────

.PHONY: install clean help

install: ## Create venv and install Python dependencies
	python3 -m venv .venv
	.venv/bin/pip install -e ".[all]"

clean: ## Remove build artifacts
	rm -rf speaktype-rust/src-tauri/target/release/bundle
	rm -rf .venv
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'
