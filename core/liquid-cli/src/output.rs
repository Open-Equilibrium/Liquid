//! Output envelope + emit + exit-code mapping.
//!
//! Every command returns one of:
//!
//! - A single [`Envelope`] (`workspace create`, `auth provision-agent`,
//!   `auth token`, `page write`, `page read`, `page undo`).
//! - An [`Envelope`] whose `data` is a `Vec<Value>` rendered as
//!   one JSON object per line — `audit list`'s NDJSON contract per
//!   `IMPLEMENTATION_PLAN.md §12`.
//!
//! Text format strips the envelope and prints a human-readable line
//! per record; errors land on stderr.

use serde::Serialize;
use serde_json::Value;

use liquid_core::LiquidError;

/// Standard envelope per `IMPLEMENTATION_PLAN.md §12` Output format.
///
/// Carries either a single `data` payload, a `records` list (NDJSON
/// emit), or an `error` message — never two at once. `ok` mirrors
/// success/failure for agent dispatch.
#[derive(Debug, Serialize)]
pub struct Envelope {
    pub ok: bool,
    pub data: Option<Value>,
    /// Only populated by NDJSON-emitting commands (`audit list`).
    /// Each element becomes one stdout line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records: Option<Vec<Value>>,
    pub error: Option<String>,
    /// Optional short message for the text formatter — when present,
    /// `Format::Text` prints this instead of formatting `data`.
    #[serde(skip_serializing)]
    pub text_summary: Option<String>,
}

impl Envelope {
    pub fn ok_data(data: Value) -> Self {
        Self {
            ok: true,
            data: Some(data),
            records: None,
            error: None,
            text_summary: None,
        }
    }

    pub fn ok_records(records: Vec<Value>) -> Self {
        Self {
            ok: true,
            data: None,
            records: Some(records),
            error: None,
            text_summary: None,
        }
    }

    pub fn with_text(mut self, summary: impl Into<String>) -> Self {
        self.text_summary = Some(summary.into());
        self
    }

    pub fn from_error(err: &LiquidError) -> Self {
        Self {
            ok: false,
            data: None,
            records: None,
            error: Some(message_for(err)),
            text_summary: None,
        }
    }

    pub fn is_ok(&self) -> bool {
        self.ok
    }
}

/// Map a `LiquidError` to its on-the-wire message.
///
/// `Forbidden` collapses to the literal string `"Forbidden"` per
/// §4.5 (never leak which failure mode tripped); `NotFound` to a
/// friendly `"Not found: <what>"`; `InvalidInput` keeps the
/// underlying message (caller-supplied — never leaks internal
/// store paths). The bats spec at `tests/cli/00_mvp_slice.bats`
/// asserts on this exact string set.
pub fn message_for(err: &LiquidError) -> String {
    match err {
        LiquidError::Forbidden => "Forbidden".to_string(),
        LiquidError::NotFound(what) => format!("Not found: {what}"),
        LiquidError::InvalidInput(msg) => msg.clone(),
    }
}

/// Process-exit code for a `LiquidError`. `Forbidden` and bridge
/// failures collapse to `1`; `InvalidInput` from misuse maps to
/// `2` (matches `EX_USAGE`). The harness can branch on these
/// without parsing stderr.
pub fn exit_code_for(err: &LiquidError) -> i32 {
    match err {
        LiquidError::InvalidInput(_) => 2,
        LiquidError::Forbidden | LiquidError::NotFound(_) => 1,
    }
}

/// Emit `envelope` to stdout in the requested format. Errors land
/// on stdout too (mirroring the §12 contract); `main` adjusts
/// the exit code.
pub fn emit(format: crate::args::Format, env: &Envelope) {
    use crate::args::Format;
    match format {
        Format::Json => emit_json(env),
        Format::Text => emit_text(env),
    }
}

fn emit_json(env: &Envelope) {
    if let Some(records) = &env.records {
        // NDJSON emit — one record per line. Top-level `ok`
        // envelope is still emitted last so a caller can grep it.
        for r in records {
            // Each line is valid JSON; `serde_json::to_string`
            // returns a single line by default.
            match serde_json::to_string(r) {
                Ok(line) => println!("{line}"),
                Err(_) => println!("{{\"error\":\"serialise failure\"}}"),
            }
        }
        return;
    }
    match serde_json::to_string(env) {
        Ok(line) => println!("{line}"),
        Err(_) => println!("{{\"ok\":false,\"error\":\"output serialise failure\"}}"),
    }
}

fn emit_text(env: &Envelope) {
    if !env.ok {
        if let Some(err) = &env.error {
            eprintln!("error: {err}");
        }
        return;
    }
    if let Some(records) = &env.records {
        for r in records {
            match serde_json::to_string(r) {
                Ok(line) => println!("{line}"),
                Err(_) => println!("(unserialisable record)"),
            }
        }
        return;
    }
    if let Some(summary) = &env.text_summary {
        println!("{summary}");
        return;
    }
    if let Some(data) = &env.data {
        match serde_json::to_string_pretty(data) {
            Ok(line) => println!("{line}"),
            Err(_) => println!("(unserialisable payload)"),
        }
    }
}
