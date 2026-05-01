//! The Tragedy of Errors: Handling AI Pipeline Failures
//!
//! Training an AI is a fragile endeavor. A missing ROM, a corrupted snapshot, or a
//! mathematically impossible configuration can all bring the agent's universe crashing down.
//!
//! This module categorizes every way the training pipeline can fail, ensuring that when
//! things break, the developer gets a clear story of *why* rather than a cryptic panic.

use std::path::PathBuf;

use burn_core::record::RecorderError;
use thiserror::Error;

/// The exhaustive list of tragedies that can befall an agent or its environment.
#[derive(Debug, Error)]
pub enum AiError {
    /// An operation that defies the physics of the simulation (e.g., trying to step before resetting).
    #[error("unsupported operation: {0}")]
    Unsupported(&'static str),
    /// The filesystem refused to grant a home for the agent's memories.
    #[error("failed to create snapshot directory '{path}': {source}")]
    SnapshotDirCreate {
        /// The path we begged the OS to create.
        path: PathBuf,
        /// The cold reality of the OS rejection.
        #[source]
        source: std::io::Error,
    },
    /// The agent's memory was too complex to be captured in JSON.
    #[error("failed to serialize snapshot bundle: {source}")]
    SnapshotSerialize {
        /// The serialization logic failure.
        #[source]
        source: serde_json::Error,
    },
    /// The agent's memory was serialized, but the disk refused to hold it.
    #[error("failed to write snapshot bundle '{path}': {source}")]
    SnapshotWrite {
        /// Where we tried to entomb the data.
        path: PathBuf,
        /// The physical IO failure.
        #[source]
        source: std::io::Error,
    },
    /// We tried to resurrect the agent, but the tomb was empty or sealed.
    #[error(
        "failed to read snapshot bundle '{path}': {source}\nHint: Ensure the file exists or check the profile configuration."
    )]
    SnapshotRead {
        /// Where we expected to find the memory.
        path: PathBuf,
        /// The reason the read failed.
        #[source]
        source: std::io::Error,
    },
    /// We resurrected the agent's memory, but it was corrupted madness.
    #[error("failed to parse snapshot bundle '{path}': {source}")]
    SnapshotParse {
        /// The file containing the corrupted memories.
        path: PathBuf,
        /// The parsing logic failure.
        #[source]
        source: serde_json::Error,
    },
    /// The snapshot comes from a timeline (version) we do not understand.
    #[error("unsupported snapshot bundle version: expected {expected}, found {found}")]
    SnapshotVersionMismatch {
        /// The timeline we know.
        expected: u32,
        /// The timeline the file belongs to.
        found: u32,
    },
    /// The filesystem refused to grant a directory for the agent's final legacy (artifacts).
    #[error("failed to create artifact directory '{path}': {source}")]
    ArtifactDirCreate {
        /// The intended resting place for the artifacts.
        path: PathBuf,
        /// The IO error blocking creation.
        #[source]
        source: std::io::Error,
    },
    /// We failed to translate the agent's final deeds into a readable JSON artifact.
    #[error("failed to serialize {kind}: {source}")]
    ArtifactSerialize {
        /// What we were trying to record (e.g. "tas movie", "episode metadata").
        kind: &'static str,
        /// The serialization error.
        #[source]
        source: serde_json::Error,
    },
    /// The agent's deeds were recorded, but the disk refused to accept the final file.
    #[error("failed to write artifact '{path}': {source}")]
    ArtifactWrite {
        /// The intended file path.
        path: PathBuf,
        /// The IO failure.
        #[source]
        source: std::io::Error,
    },
    /// The requested name prefix for saving checkpoints contained illegal or dangerous characters.
    #[error("invalid artifact prefix '{prefix}'")]
    ArtifactPrefixInvalid {
        /// The offending prefix string.
        prefix: String,
    },
    /// The Burn neural network framework failed to write its synaptic weights to disk.
    #[error("failed to save checkpoint '{path}': {source}")]
    CheckpointSave {
        /// Where the weights were supposed to go.
        path: PathBuf,
        /// The internal error from Burn's recorder.
        #[source]
        source: RecorderError,
    },
    /// The Burn neural network framework failed to read its synaptic weights from disk.
    #[error("failed to load checkpoint '{path}': {source}")]
    CheckpointLoad {
        /// Where we looked for the weights.
        path: PathBuf,
        /// The internal error from Burn's recorder.
        #[source]
        source: RecorderError,
    },
    /// We could not locate or open the sacred cartridge (ROM) the agent is supposed to inhabit.
    #[error(
        "failed to read ROM '{path}': {source}\nHint: Ensure the ROM file exists and is accessible."
    )]
    RomRead {
        /// The path we expected the ROM to be at.
        path: PathBuf,
        /// The IO error preventing access.
        #[source]
        source: std::io::Error,
    },
    /// The ROM loaded does not perfectly match the universe the snapshot was created in.
    /// A single mismatched byte could cause a catastrophic butterfly effect.
    #[error("ROM hash mismatch: expected {expected}, found {found}")]
    RomHashMismatch {
        /// The expected cryptographic signature.
        expected: String,
        /// The actual signature of the loaded file.
        found: String,
    },
}
