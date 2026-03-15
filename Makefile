.PHONY: build release lint test test-e2e clean install verify-raft help

all: lint test build ## Run lint, tests, and build

help: ## Show this message
	@echo "🛠️ Dev Commands\n"
	@grep -E '^[a-zA-Z_0-9-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

build: ## Build debug binary
	cargo build

release: ## Build optimized release binary
	cargo build --release

lint: ## Check formatting, run clippy, lint P examples
	cargo fmt -- --check
	cargo clippy --all-targets -- -D warnings
	cargo run -q -- format --check examples/raft
	cargo run -q -- lint examples/raft

test: ## Run all test suites
	cargo test

test-e2e: ## Run regression tests only
	cargo test --test regression

clean: ## Remove build artifacts
	cargo clean

install: release ## Build release and install to ~/.cargo/bin
	cargo install --path .

verify-raft: ## Run model checker on the Raft example
	cargo run -q -- verify examples/raft -t TestRaft