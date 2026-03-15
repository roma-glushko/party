.PHONY: build release lint test test-all fmt fmt-check clean install help

# Default target
all: fmt-check lint test build

# Build debug binary
build:
	cargo build

# Build optimized release binary
release:
	cargo build --release

# Run all lints (Rust code + P examples)
lint:
	cargo clippy --all-targets -- -D warnings
	cargo run -q -- lint examples/raft

# Run regression tests
test:
	cargo test --test regression

# Run all test suites
test-all:
	cargo test

# Format Rust code
fmt:
	cargo fmt

# Check Rust formatting (for CI)
fmt-check:
	cargo fmt -- --check

# Format P examples
fmt-p:
	cargo run -q -- format examples/raft

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
	@echo "  lint        Run clippy and lint P examples"
	@echo "  test        Run regression tests (409/412)"
	@echo "  test-all    Run all test suites (regression + formatter + trace + replay)"
	@echo "  fmt         Format Rust source code"
	@echo "  fmt-check   Check Rust formatting (CI)"
	@echo "  fmt-p       Format P example files"
	@echo "  clean       Remove build artifacts"
	@echo "  install     Build release and install to ~/.cargo/bin"
	@echo "  verify-raft Run model checker on the Raft example"
	@echo "  help        Show this message"
