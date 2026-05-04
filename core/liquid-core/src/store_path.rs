use serde::{Deserialize, Serialize};

use crate::{LiquidError, Result};

/// A workspace-relative path. UTF-8, forward-slash separated, never absolute,
/// never containing `.` / `..` / empty segments, never containing backslashes
/// or NUL bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StorePath(String);

impl StorePath {
    pub fn new(path: impl Into<String>) -> Result<Self> {
        let path = path.into();
        if path.is_empty() {
            return Err(LiquidError::InvalidInput("path is empty".into()));
        }
        if path.starts_with('/') {
            return Err(LiquidError::InvalidInput(format!(
                "path must be workspace-relative, got absolute: {path}"
            )));
        }
        for segment in path.split('/') {
            if segment.is_empty() {
                return Err(LiquidError::InvalidInput(format!(
                    "path contains empty segment: {path}"
                )));
            }
            if segment == "." || segment == ".." {
                return Err(LiquidError::InvalidInput(format!(
                    "path contains forbidden segment '{segment}': {path}"
                )));
            }
            if segment.bytes().any(|b| b == b'\\' || b == 0) {
                return Err(LiquidError::InvalidInput(format!(
                    "path segment contains illegal character: {segment}"
                )));
            }
        }
        Ok(Self(path))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for StorePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
