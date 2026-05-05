/// Reject a request unless the principal has the required permission.
///
/// Expands to an `await` on [`PermissionIndex::check`] and an early
/// `return Err(LiquidError::Forbidden)` if the check returns `false`.
/// The enclosing function must therefore be `async` and return a
/// `Result<_, LiquidError>` (or any error type that `From<LiquidError>`
/// converts into).
///
/// This is the canonical entrypoint for permission gating at every
/// `liquid-sdk-bridge` and CLI call site (CLAUDE.md, rule 4: "Permission
/// check is always first").
///
/// # Example
///
/// ```ignore
/// use liquid_core::{Action, AppInstanceId, PrincipalId, Resource, Result};
/// use liquid_permissions::{require_permission, InMemoryPermissionIndex};
///
/// async fn write_page(
///     index: &InMemoryPermissionIndex,
///     principal: PrincipalId,
///     app: AppInstanceId,
/// ) -> Result<()> {
///     require_permission!(index, principal, Action::Write, Resource::AppInstance(app));
///     // ... actual write logic ...
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! require_permission {
    ($index:expr, $principal:expr, $action:expr, $resource:expr $(,)?) => {{
        let __resource: $crate::__macro_support::Resource = $resource;
        let __allowed: bool =
            $crate::PermissionIndex::check($index, $principal, $action, __resource).await?;
        if !__allowed {
            return ::core::result::Result::Err($crate::__macro_support::LiquidError::Forbidden);
        }
    }};
}
