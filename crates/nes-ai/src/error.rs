//! Error types for the `nes-ai` crate.
//!
//! This module defines the [`AiError`] enum which encompasses all failure
//! modes that can occur during RL training, environment execution, or
//! checkpointing.

use std::path::PathBuf;

use burn_core::record::RecorderError;
use thiserror::Error;

/// Core error type representing failures across the AI pipeline.
///
/// Handles everything from unsupported simulation states and file I/O
/// failures to artifact serialization errors and ROM hash mismatches.
#[derive(Debug, Error)]
pub enum AiError {
    /// The requested operation or configuration is not supported.
    #[error("unsupported operation: {0}")]
    Unsupported(&'static str),
    /// Failed to create a directory for writing a snapshot bundle.
    #[error("failed to create snapshot directory '{path}': {source}")]
    SnapshotDirCreate {
        /// The path where creation failed.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },
    /// Failed to serialize the metadata for a snapshot bundle.
    #[error("failed to serialize snapshot bundle: {source}")]
    SnapshotSerialize {
        /// The underlying serialization error.
        #[source]
        source: serde_json::Error,
    },
    /// Failed to write snapshot data to disk.
    #[error("failed to write snapshot bundle '{path}': {source}")]
    SnapshotWrite {
        /// The path that failed to write.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },
    /// Failed to read snapshot data from disk.
    #[error("failed to read snapshot bundle '{path}': {source}")]
    SnapshotRead {
        /// The path that failed to read.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },
    /// Failed to parse the metadata JSON from a snapshot bundle.
    #[error("failed to parse snapshot bundle '{path}': {source}")]
    SnapshotParse {
        /// The path that failed parsing.
        path: PathBuf,
        /// The underlying deserialization error.
        #[source]
        source: serde_json::Error,
    },
    /// Snapshot bundle version is not supported by this version of `nes-ai`.
    #[error("unsupported snapshot bundle version: expected {expected}, found {found}")]
    SnapshotVersionMismatch {
        /// The version expected by the crate.
        expected: u32,
        /// The actual version found in the snapshot file.
        found: u32,
    },
    /// Failed to create a directory for training artifacts (videos, JSON logs).
    #[error("failed to create artifact directory '{path}': {source}")]
    ArtifactDirCreate {
        /// The path where creation failed.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },
    /// Failed to serialize a training artifact (e.g. metadata JSON).
    #[error("failed to serialize {kind}: {source}")]
    ArtifactSerialize {
        /// The type of artifact being serialized.
        kind: &'static str,
        /// The underlying serialization error.
        #[source]
        source: serde_json::Error,
    },
    /// Failed to write a training artifact to disk.
    #[error("failed to write artifact '{path}': {source}")]
    ArtifactWrite {
        /// The path that failed to write.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },
    /// Invalid prefix provided for artifact file generation.
    #[error("invalid artifact prefix '{prefix}'")]
    ArtifactPrefixInvalid {
        /// The invalid prefix string.
        prefix: String,
    },
    /// Failed to save a ML model checkpoint to disk.
    #[error("failed to save checkpoint '{path}': {source}")]
    CheckpointSave {
        /// The path where the checkpoint save failed.
        path: PathBuf,
        /// The underlying recorder error.
        #[source]
        source: RecorderError,
    },
    /// Failed to load a ML model checkpoint from disk.
    #[error("failed to load checkpoint '{path}': {source}")]
    CheckpointLoad {
        /// The path where the checkpoint load failed.
        path: PathBuf,
        /// The underlying recorder error.
        #[source]
        source: RecorderError,
    },
    /// Failed to read an NES ROM file from disk.
    #[error("failed to read ROM '{path}': {source}")]
    RomRead {
        /// The path that failed to read.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },
    /// The loaded ROM does not match the hash required by the environment snapshot.
    #[error("ROM hash mismatch: expected {expected}, found {found}")]
    RomHashMismatch {
        /// The SHA-256 hash expected by the snapshot.
        expected: String,
        /// The SHA-256 hash calculated from the ROM.
        found: String,
    },
}
