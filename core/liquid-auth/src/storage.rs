use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use liquid_core::{LiquidError, PrincipalId, Result, WorkspaceId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// On-disk record for a registered user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct UserRecord {
    pub id: Uuid,
    pub username: String,
    /// `argon2id$...` PHC string; never the raw password.
    pub password_hash: String,
}

/// On-disk record for a provisioned agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct AgentRecord {
    pub id: Uuid,
    pub name: String,
    pub workspace_id: Uuid,
    /// String form of the authorising principal (e.g. `user:UUID`).
    pub authorized_by: String,
    pub created_unix: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct UsersFile {
    #[serde(default)]
    users: Vec<UserRecord>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct AgentsFile {
    #[serde(default)]
    agents: Vec<AgentRecord>,
}

pub(crate) fn users_path(root: &Path) -> PathBuf {
    root.join("users.toml")
}

pub(crate) fn agents_path(root: &Path) -> PathBuf {
    root.join("agents.toml")
}

pub(crate) fn load_users(root: &Path) -> Result<Vec<UserRecord>> {
    let path = users_path(root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path).map_err(|e| io_err(&e))?;
    let parsed: UsersFile = toml::from_str(&text)
        .map_err(|e| LiquidError::InvalidInput(format!("users.toml parse error: {e}")))?;
    Ok(parsed.users)
}

pub(crate) fn save_users(root: &Path, users: &[UserRecord]) -> Result<()> {
    let payload = UsersFile {
        users: users.to_vec(),
    };
    let text = toml::to_string(&payload)
        .map_err(|e| LiquidError::InvalidInput(format!("users.toml write error: {e}")))?;
    atomic_write(&users_path(root), text.as_bytes())
}

pub(crate) fn load_agents(root: &Path) -> Result<Vec<AgentRecord>> {
    let path = agents_path(root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path).map_err(|e| io_err(&e))?;
    let parsed: AgentsFile = toml::from_str(&text)
        .map_err(|e| LiquidError::InvalidInput(format!("agents.toml parse error: {e}")))?;
    Ok(parsed.agents)
}

pub(crate) fn save_agents(root: &Path, agents: &[AgentRecord]) -> Result<()> {
    let payload = AgentsFile {
        agents: agents.to_vec(),
    };
    let text = toml::to_string(&payload)
        .map_err(|e| LiquidError::InvalidInput(format!("agents.toml write error: {e}")))?;
    atomic_write(&agents_path(root), text.as_bytes())
}

pub(crate) fn ensure_root(root: &Path) -> Result<()> {
    fs::create_dir_all(root).map_err(|e| io_err(&e))
}

/// Atomic write + Unix mode 0600 clamp.
///
/// `users.toml` holds the Argon2id PHC string for every registered
/// user; `agents.toml` holds the agent registry (`authorized_by`
/// principal IDs + names). Both must NOT be readable by other local
/// users — the default umask (often 0022) would otherwise leave the
/// PHC string world-readable, enabling offline dictionary attacks.
fn atomic_write(target: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| io_err(&e))?;
    }
    let mut tmp = target.to_path_buf();
    tmp.set_extension("toml.tmp");
    {
        let mut f = fs::File::create(&tmp).map_err(|e| io_err(&e))?;
        f.write_all(bytes).map_err(|e| io_err(&e))?;
        f.sync_all().map_err(|e| io_err(&e))?;
    }
    fs::rename(&tmp, target).map_err(|e| io_err(&e))?;
    restrict_perms(target)
}

#[cfg(unix)]
fn restrict_perms(target: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(target, std::fs::Permissions::from_mode(0o600)).map_err(|e| io_err(&e))
}

#[cfg(not(unix))]
fn restrict_perms(_target: &Path) -> Result<()> {
    // Windows ACL inheritance from the parent directory; tightening
    // the file ACL explicitly is out of scope for Phase 1.
    Ok(())
}

fn io_err(e: &std::io::Error) -> LiquidError {
    LiquidError::InvalidInput(format!("auth storage I/O error: {e}"))
}

pub(crate) fn principal_to_string(p: PrincipalId) -> String {
    match p {
        PrincipalId::User(id) => format!("user:{id}"),
        PrincipalId::Agent(id) => format!("agent:{id}"),
    }
}

pub(crate) fn workspace_uuid(ws: WorkspaceId) -> Uuid {
    ws.0
}
