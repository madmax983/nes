use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("unsupported operation: {0}")]
    Unsupported(&'static str),
}
