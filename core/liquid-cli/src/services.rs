//! Build the [`BridgeServices`] composition root from `$LIQUID_HOME`.
//!
//! Layout under `$LIQUID_HOME` (defaults to `$HOME/.liquid`):
//!
//! ```text
//! $LIQUID_HOME/
//!   auth/      — LocalIdentityProvider (users.toml + agents.toml)
//!   vcs/       — FilesystemContentStore (per-workspace dirs)
//!   perm/      — FilesystemPermissionIndex (per-workspace perms.toml)
//!   registry/  — FilesystemWorkspaceRegistry (workspaces.toml)
//!   secret     — HMAC-SHA256 key bytes (≥16; first run generates 32)
//!   token      — default bootstrap bearer token (one line; first run only)
//! ```
//!
//! Every Phase-1 subprocess re-opens the four backends, so the
//! CLI is stateless beyond what lives in `$LIQUID_HOME`.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use liquid_auth::LocalIdentityProvider;
use liquid_core::{LiquidError, Result};
use liquid_permissions::FilesystemPermissionIndex;
use liquid_sdk_bridge::{BridgeServices, FilesystemWorkspaceRegistry};
use liquid_vcs::FilesystemContentStore;

/// The concrete [`BridgeServices`] type the CLI assembles. Pinned
/// here so the cmd / token modules can name it once.
pub type CliServices = BridgeServices<
    FilesystemContentStore,
    FilesystemPermissionIndex,
    LocalIdentityProvider,
    FilesystemWorkspaceRegistry,
>;

/// Resolve `$LIQUID_HOME`, defaulting to `$HOME/.liquid`. Returns
/// `InvalidInput` if neither env var is set (e.g. a non-login shell
/// in CI without `HOME`).
pub fn liquid_home() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("LIQUID_HOME") {
        return Ok(PathBuf::from(dir));
    }
    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home).join(".liquid"));
    }
    Err(LiquidError::InvalidInput(
        "neither LIQUID_HOME nor HOME is set — cannot locate state root".into(),
    ))
}

/// Build the four backends rooted at `home`. Idempotent — each
/// backend's `open` constructor creates the on-disk layout if
/// absent. The HMAC secret is generated on first call and reused
/// for every subsequent process.
pub fn build_services(home: &Path) -> Result<CliServices> {
    fs::create_dir_all(home).map_err(|e| io_err("create home", &e))?;
    let secret = ensure_secret(home)?;
    let auth = LocalIdentityProvider::new(home.join("auth"), &secret)?;
    let store = FilesystemContentStore::open(home.join("vcs"))?;
    let permissions = FilesystemPermissionIndex::open(home.join("perm"))?;
    let registry = FilesystemWorkspaceRegistry::open(home.join("registry"))?;
    Ok(BridgeServices {
        store: Arc::new(store),
        permissions: Arc::new(permissions),
        identity: Arc::new(auth),
        registry: Arc::new(registry),
    })
}

/// Load the HMAC secret from `<home>/secret`; generate + persist a
/// fresh 32-byte secret on first run. We use two UUID v4s
/// concatenated for the bytes — `uuid` is already a workspace
/// dependency and pulls `getrandom`, so we avoid adding a separate
/// CSPRNG crate.
fn ensure_secret(home: &Path) -> Result<Vec<u8>> {
    let path = home.join("secret");
    if path.exists() {
        let bytes = fs::read(&path).map_err(|e| io_err("read secret", &e))?;
        if bytes.len() < 16 {
            return Err(LiquidError::InvalidInput(format!(
                "secret at {} is too short: {} bytes (need ≥16)",
                path.display(),
                bytes.len()
            )));
        }
        return Ok(bytes);
    }
    let mut secret = Vec::with_capacity(32);
    secret.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    secret.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    atomic_write(&path, &secret)?;
    Ok(secret)
}

/// `<home>/secret` permissions write — atomic so a partial write
/// cannot leave a half-secret on disk.
pub(crate) fn atomic_write(target: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| io_err("create parent", &e))?;
    }
    let mut tmp = target.to_path_buf();
    let mut tmp_name = tmp
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    tmp_name.push(".tmp");
    tmp.set_file_name(tmp_name);
    {
        let mut f = fs::File::create(&tmp).map_err(|e| io_err("create tmp", &e))?;
        f.write_all(bytes).map_err(|e| io_err("write tmp", &e))?;
        f.sync_all().map_err(|e| io_err("sync tmp", &e))?;
    }
    fs::rename(&tmp, target).map_err(|e| io_err("rename", &e))
}

fn io_err(stage: &str, e: &std::io::Error) -> LiquidError {
    LiquidError::InvalidInput(format!("cli services I/O ({stage}): {e}"))
}
