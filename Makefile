.PHONY: build release lint test test-all clean install help

# Default target
all: lint test build

# Build debug binary
build:
	cargo build

# Build optimized release binary
release:
	cargo build --release

# Lint everything: format check, clippy, P examples
lint:
	cargo fmt -- --check
	cargo clippy --all-targets -- -D warnings
	cargo run -q -- format --check examples/raft
	cargo run -q -- lint examples/raft

# Run regression tests
test:
	cargo test --test regression

# Run all test suites
test-all:
	cargo test

# Clean build artifacts
clean:
	cargo clean

# Install party binary to ~/.cargo/bin
install: release
	cargo install --path .

# Verify the raft example
verify-raft:
	cargo run -q -- verify examples/raft -t TestRaft

help:
	@echo "Party — P lAnguage in RusT"
	@echo ""
	@echo "Targets:"
	@echo "  build       Build debug binary"
	@echo "  release     Build optimized release binary"
	@echo "  lint        Check formatting, run clippy, lint P examples"
	@echo "  test        Run regression tests (409/412)"
	@echo "  test-all    Run all test suites (regression + formatter + trace + replay)"
	@echo "  clean       Remove build artifacts"
	@echo "  install     Build release and install to ~/.cargo/bin"
	@echo "  verify-raft Run model checker on the Raft example"
	@echo "  help        Show this message"
