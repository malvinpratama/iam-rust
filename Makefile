.PHONY: build test fmt clippy up down logs smoke

build:
	cargo build --workspace

test:
	cargo test --workspace

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets

## Run the full stack via docker-compose
up:
	cd deploy && [ -f .env ] || cp .env.example .env
	cd deploy && docker compose up --build -d

down:
	cd deploy && docker compose down -v

logs:
	cd deploy && docker compose logs -f

## End-to-end smoke test against the running gateway
smoke:
	./scripts/smoke.sh http://localhost:8080
