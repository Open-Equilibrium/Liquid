//! `liquid` agent CLI binary.
//!
//! Phase 1 scaffold. The MVP-slice subcommands land in M6.5
//! (`IMPLEMENTATION_PLAN.md` §5.6, TASK-008); the full §12 grammar
//! ships with M7 (`IMPLEMENTATION_PLAN.md` §5.8, TASK-009). Until
//! M6.5 closes, the binary exits 64 (`EX_USAGE`) with a pointer to
//! the spec so a curious agent does not silently get an empty CLI.

fn main() {
    eprintln!(
        "liquid CLI: not yet implemented \
         (minimum surface in M6.5 — see IMPLEMENTATION_PLAN.md §5.6 / TASK-008; \
         full grammar in M7 — see §5.8 / TASK-009)"
    );
    std::process::exit(64); // EX_USAGE
}
