//! Subcommand dispatch + per-command handlers.

mod audit;
mod auth;
mod page;
mod parse;
mod workspace;

use liquid_core::Result;

use crate::args::{AuditCmd, AuthCmd, Cli, Commands, PageCmd, WorkspaceCmd};
use crate::output::Envelope;
use crate::services;

/// Single entry point for every subcommand. Returns the
/// rendered envelope on success; errors propagate so `main` can
/// map them to exit codes.
pub async fn dispatch(cli: Cli) -> Result<Envelope> {
    let home = services::liquid_home()?;
    let services = services::build_services(&home)?;
    match cli.command {
        Commands::Workspace { action } => match action {
            WorkspaceCmd::Create { name } => workspace::create(&services, &home, name).await,
        },
        Commands::Auth { action } => match action {
            AuthCmd::ProvisionAgent {
                name,
                workspace,
                role,
                scope,
            } => {
                auth::provision_agent(&services, &home, name, &workspace, &role, scope.as_deref())
                    .await
            }
            AuthCmd::Token => auth::token(&home),
        },
        Commands::Page { action } => match action {
            PageCmd::Write {
                path,
                workspace,
                data,
                file,
                message,
            } => page::write(&services, &home, &path, &workspace, data, file, message).await,
            PageCmd::Read { path, workspace } => {
                page::read(&services, &home, &path, &workspace).await
            }
            PageCmd::Undo {
                path,
                workspace,
                op,
            } => page::undo(&services, &home, &path, &workspace, &op).await,
        },
        Commands::Audit { action } => match action {
            AuditCmd::List {
                workspace,
                principal,
                action,
                since,
                limit,
            } => {
                audit::list(
                    &services,
                    &home,
                    &workspace,
                    principal.as_deref(),
                    action.as_deref(),
                    since,
                    limit,
                )
                .await
            }
        },
    }
}
