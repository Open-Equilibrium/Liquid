use serde::{Deserialize, Serialize};

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

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
