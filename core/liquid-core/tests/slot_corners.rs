//! Coverage backfill for `liquid_core::slot` — exercises the surface
//! the M1 `tests/integration.rs` happy-path suite left at <100%:
//!
//!   - `SlotName::Display` (used when a slot identifier needs to land
//!     in a log line or an error message).
//!   - The two distinct error-message branches inside `SlotName::new`,
//!     asserted on both their variant AND the human-readable text so
//!     the substring contract callers depend on (the failing input
//!     appearing verbatim in the message) cannot silently regress.
//!
//! Mirrors the per-milestone style of `tests/integration.rs` — small,
//! focused, no shared fixtures.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use liquid_core::{LiquidError, SlotName};

#[test]
fn slot_name_display_round_trips_input() {
    let s = SlotName::new("sheet:selectedRange").expect("valid");
    // Display must equal the canonical input bytes — callers depend on
    // this for log lines and error messages.
    assert_eq!(format!("{s}"), "sheet:selectedRange");
}

#[test]
fn slot_name_error_for_bad_shape_quotes_input() {
    let err = SlotName::new("no-colon-here").expect_err("must reject");
    match err {
        LiquidError::InvalidInput(msg) => {
            assert!(
                msg.contains("no-colon-here"),
                "shape error must quote the offending input: {msg}"
            );
            assert!(
                msg.contains("<namespace>:<descriptor>"),
                "shape error must name the expected grammar: {msg}"
            );
        }
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

#[test]
fn slot_name_error_for_illegal_chars_quotes_input() {
    let err = SlotName::new("sheet:selected range").expect_err("must reject");
    match err {
        LiquidError::InvalidInput(msg) => {
            assert!(
                msg.contains("sheet:selected range"),
                "character-class error must quote the offending input: {msg}"
            );
            assert!(
                msg.contains("[A-Za-z0-9_]+"),
                "character-class error must name the allowed class: {msg}"
            );
        }
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}
