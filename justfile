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

# Run all Rust tests, piping output through filter-test-output.sh
# (compact failure-oriented summary; raw log under .ai/artifacts/logs/).
test-rust-filtered:
    bash -c 'set -o pipefail; cargo test --manifest-path core/Cargo.toml --workspace 2>&1 | .claude/hooks/filter-test-output.sh'

# Lint Rust (format check + clippy)
# `cargo fmt` needs `--all` when `--manifest-path` is set; without it
# rustfmt 1.8+ errors with "Failed to find targets". Matches the CI
# invocation in .github/workflows/ci.yml (which uses
# `working-directory: core` + `cargo fmt --all --check`).
lint-rust:
    cargo fmt --all --manifest-path core/Cargo.toml --check
    cargo clippy --manifest-path core/Cargo.toml --all-targets -- -D warnings

# Lint Rust, piping clippy output through filter-test-output.sh
lint-rust-filtered:
    bash -c 'set -o pipefail; cargo fmt --all --manifest-path core/Cargo.toml --check 2>&1 | .claude/hooks/filter-test-output.sh'
    bash -c 'set -o pipefail; cargo clippy --manifest-path core/Cargo.toml --all-targets -- -D warnings 2>&1 | .claude/hooks/filter-test-output.sh'

# Auto-fix Rust formatting
fmt-rust:
    cargo fmt --all --manifest-path core/Cargo.toml

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
# Skipped before M6 scaffolds `app/` — matches lefthook + CI behaviour.
test-app:
    @sh -c '[ -f app/pubspec.yaml ] || { echo "(skip: app/pubspec.yaml not yet — see IMPLEMENTATION_PLAN.md §5.7)"; exit 0; }; cd app && flutter test --coverage'

# Lint the Flutter app
lint-app:
    @sh -c '[ -f app/pubspec.yaml ] || { echo "(skip: app/pubspec.yaml not yet — see IMPLEMENTATION_PLAN.md §5.7)"; exit 0; }; cd app && dart format --output=none --set-exit-if-changed . && flutter analyze'

# Auto-fix Flutter app formatting
fmt-app:
    @sh -c '[ -f app/pubspec.yaml ] || { echo "(skip: app/pubspec.yaml not yet — see IMPLEMENTATION_PLAN.md §5.7)"; exit 0; }; cd app && dart format .'

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
# Skipped before M6 scaffolds `sdk/liquid_sdk/` — matches lefthook + CI behaviour.
test-sdk:
    @sh -c '[ -f sdk/liquid_sdk/pubspec.yaml ] || { echo "(skip: sdk/liquid_sdk/pubspec.yaml not yet — see IMPLEMENTATION_PLAN.md §5.7)"; exit 0; }; cd sdk/liquid_sdk && flutter test --coverage'

# Run SDK tests, piping output through filter-test-output.sh
test-sdk-filtered:
    @sh -c '[ -f sdk/liquid_sdk/pubspec.yaml ] || { echo "(skip: sdk/liquid_sdk/pubspec.yaml not yet)"; exit 0; }; (cd sdk/liquid_sdk && flutter test) 2>&1 | .claude/hooks/filter-test-output.sh'

# Lint the SDK
lint-sdk:
    @sh -c '[ -f sdk/liquid_sdk/pubspec.yaml ] || { echo "(skip: sdk/liquid_sdk/pubspec.yaml not yet — see IMPLEMENTATION_PLAN.md §5.7)"; exit 0; }; cd sdk/liquid_sdk && dart format --output=none --set-exit-if-changed . && flutter analyze'

# Auto-fix SDK formatting
fmt-sdk:
    @sh -c '[ -f sdk/liquid_sdk/pubspec.yaml ] || { echo "(skip: sdk/liquid_sdk/pubspec.yaml not yet)"; exit 0; }; cd sdk/liquid_sdk && dart format .'

# ── CLI bats tests ────────────────────────────────────────────────────────────

# Run CLI integration tests (requires bats)
test-cli:
    bats tests/cli/

# Run CLI integration tests, piping output through filter-test-output.sh
test-cli-filtered:
    bash -c 'set -o pipefail; bats tests/cli/ 2>&1 | .claude/hooks/filter-test-output.sh'

# ── Repo-local Claude Code config ─────────────────────────────────────────────

# Sanity-check .claude/ configuration (settings, hooks, CLAUDE.md refs)
ai-check:
    ./scripts/ai-check.sh

# Sync-docs gate — milestone state agrees across README/TASKS/CHANGELOG/plan
sync-docs-check:
    ./scripts/sync-docs-check.sh

# cargo-deny gate — advisories, licenses, bans, sources.
# Mirrors the EmbarkStudios/cargo-deny-action invocation in
# .github/workflows/audit.yml so local runs and CI fail on identical input.
# Requires `cargo install --locked cargo-deny` (or the binary from
# https://github.com/EmbarkStudios/cargo-deny/releases).
deny-check:
    cargo deny --manifest-path core/Cargo.toml check --config deny.toml

# Atomically bump LIQUID_VERSION across `core/Cargo.toml`:
# `[workspace.package].version` AND every `liquid-* = { path =
# "...", version = "..." }` literal in `[workspace.dependencies]`.
# Wrapped script lives at `scripts/bump-version.sh`; bats coverage
# at `tests/cli/02_bump_version.bats`. Run before `cargo release`
# at every Phase milestone so path-dep version literals never drift.
bump-version new:
    ./scripts/bump-version.sh {{new}}

# ── Combined ──────────────────────────────────────────────────────────────────

# Run ALL tests across every layer
test: test-rust test-app test-sdk test-cli

# Run ALL linters across every layer
lint: lint-rust lint-app lint-sdk

# Auto-fix ALL formatting
fmt: fmt-rust fmt-app fmt-sdk

# Full pre-push validation (same as CI: lint → test → cargo-deny → tarpaulin)
check: lint test deny-check coverage-check

# Remove all transient on-disk state the walkthroughs / examples
# write under `<temp_dir>/liquid-m*-walkthrough`. The walkthroughs
# themselves use `std::env::temp_dir()` (which is `/tmp` on Linux,
# `/private/tmp` on macOS — honour `$TMPDIR` when set), and keep
# their state after exit for human inspection (see
# `core/liquid-vcs/examples/m2_walkthrough.rs` +
# `core/liquid-permissions/examples/m3_walkthrough.rs`). This verb
# is the explicit cleanup an agent runs before switching milestones
# or moving to a fresh container. Idempotent — no-op on a clean tree.
clean-walkthroughs:
    bash -c 'rm -rf "${TMPDIR:-/tmp}"/liquid-m*-walkthrough'

# Project-wide clean: removes every generated coverage report (Rust
# tarpaulin HTML at repo-root `coverage/`, Flutter lcov at
# `app/coverage/` and `sdk/liquid_sdk/coverage/`) and every
# walkthrough's transient state. Does NOT touch cargo's target/ tree
# (use `cargo clean` for that — the workspace `target/` is large and
# expensive to rebuild, so an explicit verb avoids "lost an hour"
# accidents).
clean: clean-walkthroughs
    rm -rf coverage/ app/coverage/ sdk/liquid_sdk/coverage/

# Reproduce the .github/workflows/ci.yml Rust job locally with one verb.
# Mirrors the workflow's `working-directory: core` plus the three exact
# command-lines its Rust matrix job runs. Bump this together with the
# `rust:` job in ci.yml so local + CI never drift.
check-ci:
    cd core && cargo fmt --all --check
    cd core && cargo clippy --workspace --all-targets --locked -- -D warnings
    cd core && cargo test --workspace --locked

# Coverage gate — runs cargo-tarpaulin across the whole Rust workspace
# and fails the build if line coverage drops below 80%. Same threshold
# as `.codecov.yml`'s `coverage.status.project.default.target: 80%`.
# Skips clean rebuilds so local re-runs stay quick.
#
# This is a LOCAL gate, not a CI mirror. CI's tarpaulin step
# (`.github/workflows/ci.yml`) uploads cobertura XML to Codecov
# (which enforces the 80% target via `.codecov.yml`); this recipe
# instead fails the local build directly via `--fail-under 80` so
# pre-push catches a coverage drop before CI does. Install with
# `scripts/setup-tooling.sh` (or
# `cargo install --locked cargo-tarpaulin --version ^0.31`).
coverage-check:
    cargo tarpaulin --manifest-path core/Cargo.toml --workspace --skip-clean --fail-under 80 --out Stdout
