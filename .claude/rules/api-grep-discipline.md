# API Grep Discipline

**Before writing code that calls a Rust API in this workspace, grep
the actual signature first.** Do not infer the signature from the
type's name, from a sibling test, or from memory of how a "similar"
trait looked elsewhere.

```sh
grep -n 'pub fn <name>\|pub trait <name>' core/<crate>/src/
```

Assumed signatures are the single most expensive class of mistake on
this codebase: every wrong assumption costs a compile-error round
trip, and most wrong assumptions cascade through 3–5 edit rounds
before they are fixed. Two minutes of grep up front replaces ten
minutes of clippy / cargo-test ping-pong.

## When to grep

Always, before the first call site, when you are:

- Calling any trait method whose signature differs across implementors
  (`PermissionIndex::check` does NOT take `WorkspaceId`; the
  in-memory and filesystem impls have identical shapes — read the
  trait, not the impl).
- Constructing or destructuring a public type from another crate
  (`Resource`, `Action`, `Binding`, `PrincipalId::User(_)`).
- Calling a `#[must_use]` builder (`with_token_lifetime`,
  `LocalIdentityProvider::new`) — wrong argument shape compiles only
  on the happy path and the error path leaks the wrong type.
- Writing a test against a type that lacks `Debug` (cannot use
  `expect_err`; must pattern-match — see
  `liquid-auth/tests/local_provider_corners.rs::new_rejects_short_hmac_secret`).
- Wiring an `async_trait` method — the macro changes the visible
  signature in ways that cargo-error messages report inconsistently
  across rustc versions.

## How to grep efficiently

```sh
# Trait surface, single crate
grep -nE 'pub (trait|fn|struct|enum) <name>' core/<crate>/src/

# Impl method on a known type
grep -nE 'impl [A-Za-z]+ for <type>|fn <method>' core/<crate>/src/

# Re-exports from a crate's lib.rs (catches the public alias path)
grep -n 'pub use' core/<crate>/src/lib.rs
```

For broader exploration (where is X defined; which files reference
Y), delegate to the `Explore` subagent — see
[`subagent-routing.md`](subagent-routing.md). Direct `grep` is for
known names.

## What "wrong" looks like in practice

Two real examples from this branch's history:

- Wrote `PermissionIndex::check(workspace, principal, action,
  &resource)` based on a sibling test using `require_permission!`,
  which expanded to a different shape. Real signature:
  `check(principal, action, resource)` — three positional args, no
  workspace. One grep on
  `core/liquid-permissions/src/index.rs` would have caught it.
- Used `Result<_, LocalIdentityProvider>::expect_err` to assert a
  rejection. `LocalIdentityProvider` does not implement `Debug`, so
  the test failed to compile with a long type-error cascade.
  `grep 'impl.*Debug' core/liquid-auth/src/local.rs` returns nothing
  — fall back to pattern-match.

## Hard rule

If you are about to write a function call or `use` statement and you
cannot quote the exact line of the source where the symbol is
defined, **stop**. Grep first. Cargo will tell you if you are wrong
eventually; grep tells you in two seconds.
