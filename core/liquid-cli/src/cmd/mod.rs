//! Subcommand dispatch + per-command handlers.

mod audit;
mod auth;
mod impersonate;
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
    // Resolve `--as` impersonation BEFORE the per-subcommand
    // dispatch so every handler sees the impersonated token. Per
    // the M7 contract this requires the caller hold `Action::Admin`
    // on the target principal's workspace; the helper validates +
    // mints a fresh short-lived token for the target.
    if let Some(target) = cli.as_principal.as_deref() {
        let new_token = impersonate::resolve(&services, &home, target).await?;
        std::env::set_var("LIQUID_TOKEN", new_token);
    }
    match cli.command {
        Commands::Workspace { action } => match action {
            WorkspaceCmd::Create { name } => workspace::create(&services, &home, name).await,
            WorkspaceCmd::List => workspace::list(&services, &home).await,
            WorkspaceCmd::Delete { id } => workspace::delete(&services, &home, &id).await,
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
            AuthCmd::Login {
                username,
                password,
                register,
            } => auth::login(&services, &home, &username, &password, register).await,
            AuthCmd::Whoami => auth::whoami(&services, &home).await,
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
            PageCmd::History {
                path,
                workspace,
                limit,
            } => page::history(&services, &home, &path, &workspace, limit).await,
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
