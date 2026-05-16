//! Data binding pub/sub broker.
//!
//! Implements [`SlotBroker`] (specified in
//! `IMPLEMENTATION_PLAN.md §4.4` + §6.2) and one Phase-2 backend,
//! [`InProcessSlotBroker`], that uses per-slot
//! `tokio::sync::broadcast` channels for the pub/sub bus and an
//! in-memory wiring table for declarative output→input edges.
//!
//! Persistence: wiring is serialised as
//! [`BindingsDocument`] for the SDK to write to
//! `.liquid/pages/<page_id>/bindings.json` per §6.2's "wiring is
//! replayed on page load" rule.
//!
//! Phase 4 (M18) adds a distributed event bus behind the same
//! trait; application code only ever sees [`SlotBroker`].

pub mod broker;

pub use broker::{
    BindingsDocument, InProcessSlotBroker, SharedBroker, SlotBroker, SlotWiring, SLOT_BUFFER_SIZE,
};
