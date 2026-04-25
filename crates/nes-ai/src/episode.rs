use std::{fs, path::PathBuf};

use nes_core::tas::{TasError, TasMovie};
use serde::Serialize;

use crate::error::AiError;

/// Standard metadata collected over the lifetime of a single training episode.
#[derive(Debug, Clone, Serialize)]
pub struct EpisodeMetadata {
    /// The profile this episode was evaluated under.
    pub profile_id: String,
    /// The unique identifier of the starting state snapshot.
    pub snapshot_id: String,
    /// The SHA-256 hash of the ROM.
    pub rom_hash: String,
    /// The total accumulated reward (return).
    pub total_reward: f32,
    /// Total number of emulator frames elapsed.
    pub episode_frames: u64,
    /// A structural hash of the emulator's final state.
    pub final_state_hash: u64,
}

/// Contains the paths to the artifacts generated during an evaluation episode.
#[derive(Debug, Clone)]
pub struct EpisodeArtifactPaths {
    /// The path to the saved TAS input replay movie.
    pub tas_json_path: PathBuf,
    /// The path to the saved episode metadata JSON file.
    pub run_json_path: PathBuf,
    /// The path to the saved macro text file (if exported).
    pub macro_txt_path: Option<PathBuf>,
}

/// A utility for writing episode recording artifacts to disk.
#[derive(Debug, Clone)]
pub struct EpisodeArtifactWriter {
    output_dir: PathBuf,
}

impl EpisodeArtifactWriter {
    /// Binds the writer to a specific output directory.
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
