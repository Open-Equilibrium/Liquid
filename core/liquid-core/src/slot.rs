use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::{LiquidError, Result};

/// A namespaced slot identifier (`<namespace>:<descriptor>`).
/// Both segments must be non-empty `[A-Za-z0-9_]+`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SlotName(String);

impl SlotName {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        let mut parts = name.split(':');
        let ns = parts.next();
        let desc = parts.next();
        let extra = parts.next();

        match (ns, desc, extra) {
            (Some(ns), Some(desc), None) if !ns.is_empty() && !desc.is_empty() => {
                if !ns.bytes().all(is_ident_byte) || !desc.bytes().all(is_ident_byte) {
                    return Err(LiquidError::InvalidInput(format!(
                        "slot name segments must match [A-Za-z0-9_]+: {name}"
                    )));
                }
                Ok(Self(name))
            }
            _ => Err(LiquidError::InvalidInput(format!(
                "slot name must match '<namespace>:<descriptor>': {name}"
            ))),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

const fn is_ident_byte(b: u8) -> bool {
    matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
}

impl std::fmt::Display for SlotName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A typed slot value. Mirrors `SlotValue` in the Dart SDK.
/// See `IMPLEMENTATION_PLAN.md` §13 (Data Binding Protocol).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum SlotValue {
    Str(String),
    Num(f64),
    Bool(bool),
    Json(serde_json::Value),
    Bytes(Bytes),
}
