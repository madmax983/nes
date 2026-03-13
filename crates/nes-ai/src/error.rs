use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("unsupported operation: {0}")]
    Unsupported(&'static str),
    #[error("failed to create snapshot directory '{path}': {source}")]
    SnapshotDirCreate {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize snapshot bundle: {source}")]
    SnapshotSerialize {
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to write snapshot bundle '{path}': {source}")]
    SnapshotWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read snapshot bundle '{path}': {source}")]
    SnapshotRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse snapshot bundle '{path}': {source}")]
    SnapshotParse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("unsupported snapshot bundle version: expected {expected}, found {found}")]
    SnapshotVersionMismatch { expected: u32, found: u32 },
    #[error("ROM hash mismatch: expected {expected}, found {found}")]
    RomHashMismatch { expected: String, found: String },
}
