use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{LiquidError, Result};

/// Hex-encoded SHA-256 of the bytes the hash represents.
/// Always 64 lowercase hex characters once validated.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentHash(String);

impl ContentHash {
    pub fn from_hex(hex: impl Into<String>) -> Result<Self> {
        let hex = hex.into();
        if hex.len() != 64 {
            return Err(LiquidError::InvalidInput(format!(
                "ContentHash must be 64 hex chars, got {}",
                hex.len()
            )));
        }
        if !hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return Err(LiquidError::InvalidInput(
                "ContentHash must be lowercase hex".into(),
            ));
        }
        Ok(Self(hex))
    }

    /// Infallible SHA-256 hash of `bytes`. The output is always a
    /// 64-character lowercase-hex string, so the invariant
    /// `from_hex` would have checked is preserved by construction.
    ///
    /// Used by the cache layer (M4 — `IMPLEMENTATION_PLAN.md` §9
    /// `liquid-cache`: "`ContentHash` is computed from the content
    /// bytes before storing") and any other call site that needs to
    /// key bytes by their digest. Centralised here so the SHA-256
    /// dependency lives in one crate and Absolute Rule 1 (no
    /// `unwrap`/`expect` outside `#[cfg(test)]`) is not bent in
    /// callers.
    #[must_use]
    pub fn of_bytes(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        let mut hex = String::with_capacity(64);
        for byte in digest {
            use std::fmt::Write as _;
            // SAFETY: writing `{:02x}` of a byte into a `String`
            // cannot fail — `String`'s `fmt::Write` impl is
            // infallible. The `let _ =` discards the
            // `Result<(), fmt::Error>` without the
            // Absolute-Rule-violating `.unwrap()` / `.expect()`.
            let _ = write!(hex, "{byte:02x}");
        }
        Self(hex)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
