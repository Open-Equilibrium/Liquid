//! Tokio runtime helper. The CLI is short-lived (one subprocess
//! per command), so a single-threaded current-thread runtime is
//! the right shape — no background workers, no shared executor.

use std::future::Future;

use liquid_core::{LiquidError, Result};

/// Build a fresh current-thread tokio runtime and `block_on(fut)`.
/// Flattens the runtime-build failure into the same `Result` shape
/// the dispatched future already returns, so `main` can treat the
/// runtime failure and any command failure identically.
pub fn block_on<F, T>(fut: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| LiquidError::InvalidInput(format!("tokio runtime: {e}")))?;
    rt.block_on(fut)
}
