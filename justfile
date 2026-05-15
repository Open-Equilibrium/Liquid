# Liquid project task runner
# Install just: https://github.com/casey/just
# Usage: just <recipe>

default:
    @just --list

# ── Setup ─────────────────────────────────────────────────────────────────────

# Wire git hooks from lefthook.yml (run once after cloning)
install-hooks:
    lefthook install
    @echo "Git hooks installed."

# Install lefthook itself (requires npm)
install-lefthook:
    npm install -g @evilmartians/lefthook
    lefthook install

# Start background services (Redis, Redpanda) — Phase 3+
services-up *profiles="phase3":
    docker compose --profile {{profiles}} up -d

# Stop background services
services-down:
    docker compose down

# ── Rust ──────────────────────────────────────────────────────────────────────

# Build the entire Rust workspace
build-rust:
    cargo build --manifest-path core/Cargo.toml --workspace

# Run all Rust tests
test-rust:
    cargo test --manifest-path core/Cargo.toml --workspace

# Lint Rust (format check + clippy)
lint-rust:
    cargo fmt --manifest-path core/Cargo.toml --check
    cargo clippy --manifest-path core/Cargo.toml --all-targets -- -D warnings

# Auto-fix Rust formatting
fmt-rust:
    cargo fmt --manifest-path core/Cargo.toml

# Generate Rust coverage report (requires cargo-tarpaulin)
coverage-rust:
    cargo tarpaulin --manifest-path core/Cargo.toml --workspace --out Html --output-dir coverage/rust/
    @echo "Report: coverage/rust/tarpaulin-report.html"

# Generate flutter_rust_bridge FFI bindings
generate-bindings:
    cd core && cargo run -p flutter_rust_bridge_codegen -- generate

# Run the agent CLI
cli *args:
    cargo run --manifest-path core/Cargo.toml -p liquid-cli -- {{args}}

# ── Flutter app ───────────────────────────────────────────────────────────────

# Run Flutter unit + widget tests (with coverage)
test-app:
    cd app && flutter test --coverage

# Lint the Flutter app
lint-app:
    cd app && dart format --output=none --set-exit-if-changed .
    cd app && flutter analyze

# Auto-fix Flutter app formatting
fmt-app:
    cd app && dart format .

# Run the app on a desktop target (linux | macos | windows)
run target="linux":
    cd app && flutter run -d {{target}}

# Build the app for a target (release)
build-app target="linux":
    cd app && flutter build {{target}} --release

# Build for ALL five platforms (requires platform SDKs installed)
build-all:
    cd app && flutter build linux --release
    cd app && flutter build windows --release
    cd app && flutter build macos --release
    cd app && flutter build ios --release --no-codesign
    cd app && flutter build appbundle --release

# ── SDK ───────────────────────────────────────────────────────────────────────

# Run SDK tests (with coverage)
test-sdk:
    cd sdk/liquid_sdk && flutter test --coverage

# Lint the SDK
lint-sdk:
    cd sdk/liquid_sdk && dart format --output=none --set-exit-if-changed .
    cd sdk/liquid_sdk && flutter analyze

# Auto-fix SDK formatting
fmt-sdk:
    cd sdk/liquid_sdk && dart format .

# ── CLI bats tests ────────────────────────────────────────────────────────────

# Run CLI integration tests (requires bats)
test-cli:
    bats tests/cli/

# ── Repo-local Claude Code config ─────────────────────────────────────────────

# Sanity-check .claude/ configuration (settings, hooks, CLAUDE.md refs)
ai-check:
    ./scripts/ai-check.sh

# ── Combined ──────────────────────────────────────────────────────────────────

# Run ALL tests across every layer
test: test-rust test-app test-sdk test-cli

# Run ALL linters across every layer
lint: lint-rust lint-app lint-sdk

# Auto-fix ALL formatting
fmt: fmt-rust fmt-app fmt-sdk

# Full pre-push validation (same as CI)
check: lint test
