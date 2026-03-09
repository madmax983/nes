use std::fs;
use std::path::{Path, PathBuf};

use nes_core::CoreSnapshot;
use serde::{Deserialize, Serialize};

const SAVE_STATE_VERSION: u32 = 1;
const SAVE_STATE_DIR: &str = "savestates";
const HASH_PREFIX_LEN: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveStateFile {
    version: u32,
    rom_hash: String,
    snapshot: CoreSnapshot,
}

#[must_use]
pub fn quicksave_path_for_rom(rom_path: &Path, rom_hash: &str) -> PathBuf {
    let stem = rom_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("rom");
    let sanitized_stem: String = stem
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let hash_prefix: String = rom_hash.chars().take(HASH_PREFIX_LEN).collect();
    PathBuf::from(SAVE_STATE_DIR).join(format!("{sanitized_stem}-{hash_prefix}.state.json"))
}

pub fn save_state_file(path: &Path, rom_hash: &str, snapshot: &CoreSnapshot) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create save-state directory '{}': {err}",
                parent.display()
            )
        })?;
    }

    let payload = SaveStateFile {
        version: SAVE_STATE_VERSION,
        rom_hash: rom_hash.to_owned(),
        snapshot: snapshot.clone(),
    };
    let encoded = serde_json::to_vec(&payload)
        .map_err(|err| format!("failed to serialize save-state payload: {err}"))?;
    fs::write(path, encoded).map_err(|err| {
        format!(
            "failed to write save-state file '{}': {err}",
            path.display()
        )
    })
}

pub fn load_state_file(path: &Path, expected_rom_hash: &str) -> Result<CoreSnapshot, String> {
    let encoded = fs::read(path)
        .map_err(|err| format!("failed to read save-state file '{}': {err}", path.display()))?;
    let payload: SaveStateFile = serde_json::from_slice(&encoded).map_err(|err| {
        format!(
            "failed to parse save-state file '{}': {err}",
            path.display()
        )
    })?;
    if payload.version != SAVE_STATE_VERSION {
        return Err(format!(
            "unsupported save-state version {} in '{}'",
            payload.version,
            path.display()
        ));
    }
    if payload.rom_hash != expected_rom_hash {
        return Err(format!(
            "ROM hash mismatch for '{}': expected {expected_rom_hash}, found {}",
            path.display(),
            payload.rom_hash
        ));
    }
    Ok(payload.snapshot)
}
