//! The Scribe's Ledger: Episode Telemetry and Artifacts
//!
//! When an AI finishes an episode—whether it died valiantly on world 1-1 or miraculously
//! beat the game—we need proof. We need to save its memory, its inputs, and its final score.
//!
//! This module acts as the historian. It scoops up the TAS input sequences and performance
//! metadata from an environment and carefully etches them onto the disk for later
//! evaluation, debugging, and bragging rights.

use std::{fs, path::PathBuf};

use nes_core::tas::{TasError, TasMovie};
use serde::Serialize;

use crate::error::AiError;

/// The final epitaph of a completed training run.
///
/// This structure holds the high-level summary of an episode's events. Was it a
/// glorious success or a tragic failure? The numbers tell the story.
#[derive(Debug, Clone, Serialize)]
pub struct EpisodeMetadata {
    /// The specific training ruleset under which the agent operated.
    pub profile_id: String,
    /// The unique identifier of the save state where the agent was born.
    pub snapshot_id: String,
    /// The cryptographic fingerprint of the universe (ROM) the agent inhabited.
    pub rom_hash: String,
    /// The final score. The ultimate measure of the agent's worth.
    pub total_reward: f32,
    /// How many subjective ticks of time the agent experienced before the end.
    pub episode_frames: u64,
    /// The final state of the universe at the moment of completion or death.
    pub final_state_hash: u64,
}

/// The physical manifestations of the agent's memory, written to the local disk.
#[derive(Debug, Clone)]
pub struct EpisodeArtifactPaths {
    /// The path to the raw input sequence (`.tas.json`), used to exactly replay the run.
    pub tas_json_path: PathBuf,
    /// The path to the run's metadata (`.run.json`), detailing score and duration.
    pub run_json_path: PathBuf,
    /// The path to a human-readable transcription of the inputs (`.macro.txt`), if available.
    pub macro_txt_path: Option<PathBuf>,
}

/// The tireless historian that writes the agent's legacy to disk.
///
/// It ensures that all files related to an episode (metadata, TAS replays, macros)
/// are written atomically. If one fails, it attempts to clean up the rest, leaving
/// no corrupted half-histories behind.
#[derive(Debug, Clone)]
pub struct EpisodeArtifactWriter {
    /// The sanctuary where all records are kept.
    output_dir: PathBuf,
}

impl EpisodeArtifactWriter {
    /// Employs a new historian assigned to the specified directory.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_ai::episode::EpisodeArtifactWriter;
    /// use std::path::PathBuf;
    ///
    /// let scribe = EpisodeArtifactWriter::new(PathBuf::from("/tmp/ai_runs"));
    /// ```
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
