# Log Volume Rule

**Any command whose stdout + stderr is expected to exceed ~50 lines
MUST be routed through one of the project's noise-reducing paths
before its output is allowed to land in the main agent thread.**

Cargo, clippy, flutter test, bats and CI log fetches all routinely
produce hundreds of lines. Pasting that volume into the main thread
burns context, drowns useful signal, and inflates downstream costs.

## What "route through" means

You have two equivalent options. Pick whichever fits the step:

1. **Pipe through the filter hook.** Use
   `.claude/hooks/filter-test-output.sh`:
   ```sh
   cargo test 2>&1 | .claude/hooks/filter-test-output.sh
   flutter test 2>&1 | .claude/hooks/filter-test-output.sh
   bats tests/cli/ 2>&1 | .claude/hooks/filter-test-output.sh
   ```
   The raw log lands in `.ai/artifacts/logs/`; only a compact
   failure-oriented summary surfaces in the thread.

   The `*-filtered` justfile recipes wrap this for you:
   ```sh
   just test-rust-filtered
   just test-sdk-filtered
   just test-cli-filtered
   just lint-rust-filtered
   ```

2. **Delegate to the `test-triager` subagent.** When the failure isn't
   a single obvious assertion — long compile-error cascades, flaky
   integration tests, mixed-layer noise — spawn `test-triager`
   (haiku, read-only). It parses the log offline and returns just the
   relevant failure, file paths, and the next smallest command to run.

## When to use which

| Situation | Use |
|---|---|
| Routine pre-push validation, single layer | `just *-filtered` |
| Ad-hoc `cargo test`, `clippy`, `bats` invocation | pipe to `filter-test-output.sh` |
| Long mixed-layer logs, multiple failures, root-cause unclear | `test-triager` subagent |
| GitHub Actions workflow logs (>50 lines from any failed step) | `.claude/scripts/gh-job-log <run_id>` (writes raw log to `.ai/artifacts/logs/`, prints last 50 lines of every failed step) |

## What "main thread" means

The main thread is what the user sees and what the next agent turn
loads into context. The `.ai/artifacts/{logs,diffs,ui}/` tree is
explicitly **not** part of that — raw logs live there, summaries live
in the chat.

`.ai/artifacts/` is git-ignored except for its `README.md` and
`.gitignore`. Treat it as scratch storage that survives across tool
calls in the same session, not as persistent project state.

## Hard rule

If you find yourself about to paste a multi-hundred-line command
output into the thread "so the user can see it", **stop**. Re-run
through the filter hook (or the triager) and quote only the
summary. The raw log path under `.ai/artifacts/logs/` is enough of a
pointer.
