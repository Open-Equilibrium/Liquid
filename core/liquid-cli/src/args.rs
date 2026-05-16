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

    /// Impersonate a named agent or principal id for this call.
    /// Accepts a bare agent name (`"worker-bot"`) — must match
    /// exactly one provisioned agent across the host — OR a
    /// principal-form string (`"a:<uuid>"` / `"u:<uuid>"`). The
    /// caller still authenticates with their own token; the
    /// bridge then issues a short-lived impersonation token for
    /// the target principal after verifying the caller holds
    /// `Action::Admin` on the target's workspace (M7 / §5.8).
    #[arg(long = "as", global = true)]
    pub as_principal: Option<String>,

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
    /// List workspaces the caller has at least Read authority on.
    /// NDJSON emit when `--format json` (one workspace per line,
    /// newest first).
    List,
    /// Delete the workspace with id `<id>`. Requires Admin on the
    /// workspace (M7 / TASK-009). Does NOT cascade-delete on-disk
    /// VCS bytes — those remain under `$LIQUID_HOME/vcs/<id>/`
    /// for forensics.
    Delete {
        /// Workspace id (uuid).
        id: String,
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
    /// Non-interactive login for a username/password pair. With
    /// `--register` first creates the user (rejects if it already
    /// exists); without it, authenticates an existing user. On
    /// success, writes the issued token to `$LIQUID_HOME/token`
    /// so subsequent commands pick it up.
    Login {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password: String,
        /// Register the user before authenticating (one-shot).
        /// Mutually exclusive with logging in as an existing
        /// user — pass without `--register` for the latter.
        #[arg(long)]
        register: bool,
    },
    /// Print the principal the active token resolves to (matches
    /// the form `liquid audit list --principal` accepts:
    /// `user:<uuid>` or `agent:<uuid>`).
    Whoami,
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
    /// Read the page at `<path>` and decode its bytes as JSON for
    /// the envelope's `data` field. Phase-1 contract: pages MUST
    /// be JSON-encoded (the `--file` body source is not exempt —
    /// it is stored verbatim, but `read` will reject non-JSON
    /// content with `InvalidInput`). A `--raw` flag for opaque
    /// bytes is a planned M7 follow-up.
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
    /// Print the operation-log entries that touched `<path>`,
    /// newest-first. NDJSON emit (`--format json`) — one record
    /// per line; pipe through `head -n 1` to get the most recent.
    History {
        path: String,
        #[arg(long)]
        workspace: String,
        /// Maximum entries to return. Default 50.
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
}

#[derive(Debug, Subcommand)]
pub enum AuditCmd {
    /// Print operation-log entries, oldest-first — pipe through
    /// `tail -n 1` to get the most recent — as one JSON object per
    /// line (NDJSON) when `--format json`.
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
