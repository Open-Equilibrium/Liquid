use thiserror::Error;

/// Workspace-wide `Result` alias. Every public function in `liquid-core`
/// returns this; downstream crates do the same and convert their domain
/// errors via `From`.
pub type Result<T> = std::result::Result<T, LiquidError>;

#[derive(Debug, Error)]
pub enum LiquidError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("forbidden")]
    Forbidden,

    #[error("not found: {0}")]
    NotFound(String),
}
