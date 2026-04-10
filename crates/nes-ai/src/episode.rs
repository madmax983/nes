//! Tracking and serialization of completed training episodes.
//!
//! This module provides the tools to take an agent's finished "run"
//! and serialize its actions, score, and environment metadata to disk.
//! These artifacts allow us to visually review the agent's progress
//! using the NES macro playback tools.

use std::{fs, path::PathBuf};

use nes_core::tas::{TasError, TasMovie};
use serde::Serialize;

use crate::error::AiError;

/// High-level summary of a completed training episode.
///
/// This metadata is recorded alongside the actual controller input movie,
/// allowing us to analyze the performance of a run without re-simulating it.
#[derive(Debug, Clone, Serialize)]
pub struct EpisodeMetadata {
    /// The ID of the environment profile used (e.g., "smb-level-1").
    pub profile_id: String,
    /// The ID of the snapshot bundle from which the episode started.
    pub snapshot_id: String,
    /// The SHA-256 hash of the ROM used during the run.
    pub rom_hash: String,
    /// The final cumulative reward achieved by the agent.
    pub total_reward: f32,
    /// The total number of frames the episode lasted before termination.
    pub episode_frames: u64,
    /// A hash of the emulator's final state to verify replay determinism.
    pub final_state_hash: u64,
}

/// The set of output files generated for a completed episode.
#[derive(Debug, Clone)]
pub struct EpisodeArtifactPaths {
    /// Path to the JSON file containing the raw `TasMovie` data.
    pub tas_json_path: PathBuf,
    /// Path to the JSON file containing the `EpisodeMetadata`.
    pub run_json_path: PathBuf,
    /// Path to the human-readable macro text file (if applicable).
    pub macro_txt_path: Option<PathBuf>,
}

/// A utility for writing episode artifacts to a specified output directory.
#[derive(Debug, Clone)]
pub struct EpisodeArtifactWriter {
    output_dir: PathBuf,
}

impl EpisodeArtifactWriter {
    /// Creates a new writer configured to output files to the given directory.
    #[must_use]
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }

    /// Writes TAS, run metadata, and optional macro playback artifacts for one episode.
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] if the prefix is invalid, the output directory cannot be
    /// created, serialization fails, or any artifact cannot be written.
    pub fn write(
        &self,
        prefix: &str,
        movie: &TasMovie,
        meta: &EpisodeMetadata,
    ) -> Result<EpisodeArtifactPaths, AiError> {
        let prefix = sanitize_prefix(prefix)?;
        fs::create_dir_all(&self.output_dir).map_err(|source| AiError::ArtifactDirCreate {
            path: self.output_dir.clone(),
            source,
        })?;

        let tas_json_path = self.output_dir.join(format!("{prefix}.tas.json"));
        let run_json_path = self.output_dir.join(format!("{prefix}.run.json"));
        let tas_json =
            serde_json::to_vec_pretty(movie).map_err(|source| AiError::ArtifactSerialize {
                kind: "tas movie",
                source,
            })?;
        let run_json =
            serde_json::to_vec_pretty(meta).map_err(|source| AiError::ArtifactSerialize {
                kind: "episode metadata",
                source,
            })?;

        let mut written_paths = Vec::<PathBuf>::new();
        write_artifact(&tas_json_path, &tas_json).map_err(|source| {
            cleanup_artifacts(&written_paths);
            AiError::ArtifactWrite {
                path: tas_json_path.clone(),
                source,
            }
        })?;
        written_paths.push(tas_json_path.clone());

        write_artifact(&run_json_path, &run_json).map_err(|source| {
            cleanup_artifacts(&written_paths);
            AiError::ArtifactWrite {
                path: run_json_path.clone(),
                source,
            }
        })?;
        written_paths.push(run_json_path.clone());

        let macro_txt_path = match movie.to_macro_script() {
            Ok(script) => {
                let path = self.output_dir.join(format!("{prefix}.macro.txt"));
                write_artifact(&path, script.as_bytes()).map_err(|source| {
                    cleanup_artifacts(&written_paths);
                    AiError::ArtifactWrite {
                        path: path.clone(),
                        source,
                    }
                })?;
                written_paths.push(path.clone());
                Some(path)
            }
            Err(TasError::Player2MacroScriptUnsupported) => None,
        };

        Ok(EpisodeArtifactPaths {
            tas_json_path,
            run_json_path,
            macro_txt_path,
        })
    }
}

fn sanitize_prefix(prefix: &str) -> Result<&str, AiError> {
    if prefix.is_empty() || prefix == "." || prefix == ".." || prefix.contains(['/', '\\', ':']) {
        return Err(AiError::ArtifactPrefixInvalid {
            prefix: prefix.to_owned(),
        });
    }

    Ok(prefix)
}

fn write_artifact(path: &PathBuf, bytes: &[u8]) -> Result<(), std::io::Error> {
    fs::write(path, bytes)
}

fn cleanup_artifacts(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}
