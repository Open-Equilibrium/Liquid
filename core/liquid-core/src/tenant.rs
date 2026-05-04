use serde::{Deserialize, Serialize};

/// Opaque per-instance configuration. Schema-validated by the SDK at install
/// time, encrypted on disk, decrypted into memory as a `serde_json::Value`.
/// See ADR-003 (tenant config is app-instance-level, not workspace-level).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TenantConfig(pub serde_json::Value);

impl TenantConfig {
    pub fn empty() -> Self {
        Self(serde_json::Value::Object(serde_json::Map::new()))
    }

    pub fn as_value(&self) -> &serde_json::Value {
        &self.0
    }
}
