use std::path::PathBuf;

use burn_core::record::RecorderError;
use thiserror::Error;

/// Error types emitted by the AI training and evaluation pipeline.
#[derive(Debug, Error)]
pub enum AiError {
    /// An operation was attempted that is not supported by the current environment or profile.
    #[error("unsupported operation: {0}")]
    Unsupported(&'static str),
    /// Failed to create the directory hierarchy for storing environment snapshots.
    #[error("failed to create snapshot directory '{path}': {source}")]
    SnapshotDirCreate {
        /// The path that could not be created.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },
    /// Failed to serialize the snapshot bundle into JSON format.
    #[error("failed to serialize snapshot bundle: {source}")]
    SnapshotSerialize {
        /// The underlying serialization error.
        #[source]
        source: serde_json::Error,
    },
    /// Failed to write the serialized snapshot bundle to disk.
    #[error("failed to write snapshot bundle '{path}': {source}")]
    SnapshotWrite {
        /// The destination path.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },
    /// Failed to read the snapshot bundle from disk.
    #[error(
        "failed to read snapshot bundle '{path}': {source}\nHint: Ensure the file exists or check the profile configuration."
    )]
    SnapshotRead {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },
    /// Failed to parse the snapshot bundle JSON data.
    #[error("failed to parse snapshot bundle '{path}': {source}")]
    SnapshotParse {
        /// The path of the corrupted snapshot file.
        path: PathBuf,
        /// The underlying JSON parsing error.
        #[source]
        source: serde_json::Error,
    },
    /// The loaded snapshot has an incompatible version number.
    #[error("unsupported snapshot bundle version: expected {expected}, found {found}")]
    SnapshotVersionMismatch {
        /// The expected version number.
        expected: u32,
        /// The version number found in the file.
        found: u32
    },
    /// Failed to create the directory for storing AI model artifacts.
    #[error("failed to create artifact directory '{path}': {source}")]
    ArtifactDirCreate {
        /// The path of the directory that failed creation.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },
    /// Failed to serialize an AI artifact into a JSON payload.
    #[error("failed to serialize {kind}: {source}")]
    ArtifactSerialize {
        /// The name or category of the artifact being serialized (e.g. "model", "config").
        kind: &'static str,
        /// The underlying `serde_json` serialization error.
        #[source]
        source: serde_json::Error,
    },
    /// Failed to write a serialized AI artifact to the local disk.
    #[error("failed to write artifact '{path}': {source}")]
    ArtifactWrite {
        /// The target path for the artifact file.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },
    /// The provided prefix used for saving multiple sequential checkpoints is invalid.
    #[error("invalid artifact prefix '{prefix}'")]
    ArtifactPrefixInvalid {
        /// The prefix that triggered the failure.
        prefix: String,
    },
    /// The burn framework failed to save the model weights/optimizer state.
    #[error("failed to save checkpoint '{path}': {source}")]
    CheckpointSave {
        /// The destination path of the checkpoint file.
        path: PathBuf,
        /// The underlying recorder error from burn.
        #[source]
        source: RecorderError,
    },
    /// The burn framework failed to load the model weights/optimizer state from disk.
    #[error("failed to load checkpoint '{path}': {source}")]
    CheckpointLoad {
        /// The target path of the checkpoint file.
        path: PathBuf,
        /// The underlying recorder error from burn.
        #[source]
        source: RecorderError,
    },
    /// Failed to read the target NES ROM from disk.
    #[error(
        "failed to read ROM '{path}': {source}\nHint: Ensure the ROM file exists and is accessible."
    )]
    RomRead {
        /// The path of the missing or unreadable ROM file.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },
    /// The ROM loaded for training did not match the expected hash.
    #[error("ROM hash mismatch: expected {expected}, found {found}")]
    RomHashMismatch {
        /// The expected SHA-256 hash required for this training profile.
        expected: String,
        /// The actual computed SHA-256 hash of the loaded ROM file.
        found: String,
    },
}
