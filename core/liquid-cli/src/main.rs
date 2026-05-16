//! `liquid` agent CLI binary.
//!
//! Implements the M6.5 subset of `IMPLEMENTATION_PLAN.md §12`:
//!
//! - `liquid workspace create <name>`
//! - `liquid auth provision-agent <name> --workspace <id> --role <role>`
//! - `liquid auth token`
//! - `liquid page write <path> --workspace <id> --data <json>`
//! - `liquid page read <path> --workspace <id>`
//! - `liquid audit list --workspace <id>`
//! - `liquid page undo <path> --op <operation-id>`
//!
//! Authentication: every command resolves a bearer token via
//! `$LIQUID_TOKEN` → `$LIQUID_HOME/token`. `workspace create` is the
//! one bootstrap exception: when no token is available it
//! auto-registers a default `cli` user, persists the issued token
//! to `$LIQUID_HOME/token`, and uses it for the create call.
//!
//! State root: `$LIQUID_HOME` (defaults to `$HOME/.liquid`). Each
//! subprocess invocation re-opens the durable backends
//! (`FilesystemContentStore` + `FilesystemPermissionIndex` +
//! `FilesystemWorkspaceRegistry` + `LocalIdentityProvider`).
//!
//! Output: `--format text|json` (env `LIQUID_FORMAT`); JSON is a
//! single-line envelope `{ "ok": bool, "data": …, "error": … }`
//! except `audit list` which emits newline-delimited records.

mod args;
mod cmd;
mod output;
mod runtime;
mod services;
mod token;

use args::{Cli, Format};
use clap::Parser;
use output::Envelope;

fn main() {
    let cli = Cli::parse();
    let format = cli.format.unwrap_or(Format::Text);
    let outcome = runtime::block_on(cmd::dispatch(cli));
    let exit_code = match outcome {
        Ok(envelope) => {
            output::emit(format, &envelope);
            i32::from(!envelope.is_ok())
        }
        Err(err) => {
            let envelope = Envelope::from_error(&err);
            output::emit(format, &envelope);
            output::exit_code_for(&err)
        }
    };
    std::process::exit(exit_code);
}
