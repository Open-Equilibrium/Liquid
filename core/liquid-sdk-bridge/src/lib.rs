//! Thin FFI surface bridging Dart (Flutter) and the Rust core.
//!
//! Every public function will check permissions first, then delegate to the
//! relevant crate (vcs / auth / permissions / cache / bindings). No business
//! logic lives here. Phase 1 scaffold; FFI bindings land in M5
//! (`IMPLEMENTATION_PLAN.md` §5.5).
