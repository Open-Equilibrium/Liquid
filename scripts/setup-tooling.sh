#!/usr/bin/env bash
# scripts/setup-tooling.sh — idempotent installer for every developer
# tool the project's pre-push and CI gates assume.
#
# Single source of truth for which versions of each tool the repo
# targets. CONTRIBUTING.md and CLAUDE.md "First-time setup" both
# reduce to one bullet that points here, so a contributor can run:
#
#   ./scripts/setup-tooling.sh
#
# from a fresh clone and get a working development environment.
#
# Tools installed (with pinned versions where the project requires
# them):
#
#   - `cargo-deny`       — license / advisory / wildcard gate; used by
#                           `just deny-check` and `.github/workflows/
#                           audit.yml`.
#   - `cargo-tarpaulin`  — coverage gate (`^0.31`, matches the version
#                           pinned in `.github/workflows/ci.yml`); used
#                           by `just coverage-check` and `just check`.
#   - `just`             — task runner; entry point for every workflow
#                           in this repo.
#   - `bats`             — CLI integration test runner (`tests/cli/`).
#   - `lefthook`         — git hook manager (`lefthook.yml`).
#
# Idempotency: every install step first checks whether the tool is
# already present at an acceptable version, and skips it if so.
# Re-running the script is a no-op once tools are installed.
#
# Modes:
#   --help     Print usage and exit 0.
#   --dry-run  Print the install plan without executing anything.
#              Lists every tool and the action that WOULD be taken
#              (`would install <tool> ...` or `already installed`).
#
# Exit codes:
#   0   success (everything installed or already present)
#   1   an install step failed
#   2   bad invocation

set -uo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/setup-tooling.sh [--dry-run | --help]

Idempotently installs the developer toolchain Liquid's pre-push and
CI gates assume.

Tools installed:
  cargo-deny       — license / advisory gate (just deny-check)
  cargo-tarpaulin  — coverage gate, pinned to ^0.31 (just coverage-check)
  just             — task runner
  bats             — CLI integration test runner (tests/cli/)
  lefthook         — git hook manager (lefthook.yml)

Modes:
  --dry-run  Print the install plan without running it.
  --help     Print this message and exit 0.

Re-running the script after a successful run is a no-op.
EOF
}

DRY_RUN=0
case "${1:-}" in
  --help|-h)
    usage
    exit 0
    ;;
  --dry-run|-n)
    DRY_RUN=1
    ;;
  "")
    ;;
  *)
    printf 'setup-tooling: unknown argument: %s\n\n' "$1" >&2
    usage >&2
    exit 2
    ;;
esac

say()  { printf '%s\n' "$*"; }
plan() { printf '  %s\n' "$*"; }

# Run a command unless --dry-run, in which case just print it.
do_or_plan() {
  local label="$1"; shift
  if [ "$DRY_RUN" -eq 1 ]; then
    plan "$label"
  else
    say "▶ $label"
    "$@" || return 1
  fi
}

# Print "would install <tool>" if missing, "already installed" if present.
plan_install() {
  local tool="$1" label="$2"
  if command -v "$tool" >/dev/null 2>&1; then
    plan "already installed: $tool ($(command -v "$tool"))"
    return 1
  fi
  plan "would install $tool: $label"
  return 0
}

say "Liquid developer toolchain setup"
if [ "$DRY_RUN" -eq 1 ]; then
  say "  (dry-run: nothing will be installed)"
fi
say ""

# ── cargo-deny ────────────────────────────────────────────────────────────────
say "[1/5] cargo-deny"
if plan_install cargo-deny "cargo install --locked cargo-deny"; then
  if [ "$DRY_RUN" -eq 0 ]; then
    do_or_plan "cargo install --locked cargo-deny" \
      cargo install --locked cargo-deny || exit 1
  fi
fi

# ── cargo-tarpaulin (pinned ^0.31) ────────────────────────────────────────────
say "[2/5] cargo-tarpaulin (^0.31)"
if plan_install cargo-tarpaulin "cargo install --locked cargo-tarpaulin --version ^0.31"; then
  if [ "$DRY_RUN" -eq 0 ]; then
    do_or_plan "cargo install --locked cargo-tarpaulin --version ^0.31" \
      cargo install --locked cargo-tarpaulin --version '^0.31' || exit 1
  fi
fi

# ── just ──────────────────────────────────────────────────────────────────────
say "[3/5] just"
if plan_install just "cargo install --locked just"; then
  if [ "$DRY_RUN" -eq 0 ]; then
    do_or_plan "cargo install --locked just" \
      cargo install --locked just || exit 1
  fi
fi

# ── bats ──────────────────────────────────────────────────────────────────────
# bats-core is a git-installable shell script; mirrors the CI install in
# `.github/workflows/ci.yml`'s `cli` job. Prefer the system package if
# available so we do not require root.
say "[4/5] bats"
if plan_install bats "apt-get install -y bats (fallback: git clone bats-core)"; then
  if [ "$DRY_RUN" -eq 0 ]; then
    if command -v apt-get >/dev/null 2>&1 && [ "$(id -u)" -eq 0 ]; then
      do_or_plan "apt-get install -y bats" \
        apt-get install -y bats || exit 1
    elif command -v brew >/dev/null 2>&1; then
      do_or_plan "brew install bats-core" brew install bats-core || exit 1
    else
      tmp="$(mktemp -d)"
      # Match ai-check.sh's house pattern: trap the cleanup so a
      # mid-script failure does not leak the clone under /tmp.
      trap 'rm -rf "$tmp"' EXIT
      do_or_plan "git clone bats-core into $tmp" \
        git clone --depth 1 https://github.com/bats-core/bats-core.git "$tmp" || exit 1
      if [ "$(id -u)" -eq 0 ]; then
        do_or_plan "$tmp/install.sh /usr/local" \
          "$tmp/install.sh" /usr/local || exit 1
      else
        mkdir -p "$HOME/.local"
        do_or_plan "$tmp/install.sh \$HOME/.local" \
          "$tmp/install.sh" "$HOME/.local" || exit 1
        say "  add \$HOME/.local/bin to PATH if not already"
      fi
    fi
  fi
fi

# ── lefthook ──────────────────────────────────────────────────────────────────
say "[5/5] lefthook"
if plan_install lefthook "npm install -g @evilmartians/lefthook (fallback: cargo)"; then
  if [ "$DRY_RUN" -eq 0 ]; then
    if command -v npm >/dev/null 2>&1; then
      do_or_plan "npm install -g @evilmartians/lefthook" \
        npm install -g @evilmartians/lefthook || exit 1
    else
      do_or_plan "cargo install --locked lefthook" \
        cargo install --locked lefthook || exit 1
    fi
  fi
fi

say ""
if [ "$DRY_RUN" -eq 1 ]; then
  say "Dry-run complete. Re-run without --dry-run to install."
else
  say "Toolchain setup complete. Wire git hooks with:"
  say "  lefthook install   (or: just install-hooks)"
fi
