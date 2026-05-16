//! Command-line argument parsing — `clap` derive surface for the
//! M6.5 subset (`IMPLEMENTATION_PLAN.md §12`).

use clap::{Parser, Subcommand, ValueEnum};

/// Output format selector. JSON is the agent-friendly default for
/// non-tty pipes; text is the default for humans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "lowercase")]
pub enum Format {
    Text,
    Json,
}

#[derive(Debug, Parser)]
#[command(
    name = "liquid",
    about = "Liquid agent CLI",
    long_about = "Liquid agent CLI — Phase-1 MVP slice (see IMPLEMENTATION_PLAN.md §12).",
    propagate_version = true,
    version,
    arg_required_else_help = true
)]
pub struct Cli {
    /// Output format. Env: `LIQUID_FORMAT`.
    #[arg(long, value_enum, global = true, env = "LIQUID_FORMAT")]
    pub format: Option<Format>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Workspace-scoped operations.
    Workspace {
        #[command(subcommand)]
        action: WorkspaceCmd,
    },
    /// Authentication and agent provisioning.
    Auth {
        #[command(subcommand)]
        action: AuthCmd,
    },
    /// Page-scoped read / write / undo.
    Page {
        #[command(subcommand)]
        action: PageCmd,
    },
    /// Audit log inspection.
    Audit {
        #[command(subcommand)]
        action: AuditCmd,
    },
}

#[derive(Debug, Subcommand)]
pub enum WorkspaceCmd {
    /// Create a new workspace. Bootstraps a default `cli` user +
    /// token under `$LIQUID_HOME` on first run.
    Create {
        /// Human-friendly workspace name.
        name: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum AuthCmd {
    /// Provision a new agent principal and issue a session token.
    /// Caller must hold a token with `Action::Admin` on the target
    /// workspace.
    #[command(name = "provision-agent")]
    ProvisionAgent {
        /// Display name for the agent (audit-log only).
        name: String,
        /// Workspace the agent will operate in.
        #[arg(long)]
        workspace: String,
        /// Role to assign on the workspace (one of `WorkspaceOwner`,
        /// `WorkspaceMember`, `AppViewer`, `AppEditor`, `Agent`).
        #[arg(long)]
        role: String,
        /// Resource UUID for scope-required roles (`AppViewer` /
        /// `AppEditor`). Omit for workspace-scope roles.
        #[arg(long)]
        scope: Option<String>,
    },
    /// Print the current bearer token (from `$LIQUID_TOKEN` or
    /// `$LIQUID_HOME/token`).
    Token,
}

#[derive(Debug, Subcommand)]
pub enum PageCmd {
    /// Atomically write `--data` (JSON) to the page at `<path>`.
    Write {
        /// Page path, e.g. `/pages/welcome`.
        path: String,
        /// Workspace id (uuid).
        #[arg(long)]
        workspace: String,
        /// JSON payload as a string. Mutually exclusive with `--file`.
        #[arg(long, conflicts_with = "file")]
        data: Option<String>,
        /// Path to a file whose contents will be written verbatim
        /// (bytes — not interpreted as JSON beyond storage).
        #[arg(long, conflicts_with = "data")]
        file: Option<String>,
        /// Commit message attributed to the caller.
        #[arg(long, default_value = "")]
        message: String,
    },
    /// Read the current bytes of the page at `<path>`.
    Read {
        path: String,
        #[arg(long)]
        workspace: String,
    },
    /// Reverse the operation identified by `--op`.
    Undo {
        path: String,
        #[arg(long)]
        workspace: String,
        /// Operation id (uuid) to invert.
        #[arg(long)]
        op: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum AuditCmd {
    /// Print operation-log entries, newest first, one JSON object
    /// per line (NDJSON) when `--format json`.
    List {
        #[arg(long)]
        workspace: String,
        /// Filter to a specific principal (`user:<uuid>` or
        /// `agent:<uuid>`).
        #[arg(long)]
        principal: Option<String>,
        /// Filter by action — one of `Create`, `Update`, `Delete`,
        /// `Undo` (`Write` is accepted as an alias for either
        /// `Create` or `Update`).
        #[arg(long)]
        action: Option<String>,
        /// Filter to entries at or after this Unix-epoch second.
        #[arg(long)]
        since: Option<u64>,
        /// Maximum entries to return. Default 50.
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
}
