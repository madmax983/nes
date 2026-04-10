use std::path::Path;
use std::{fmt::Write as _, fs};

use nes_core::CoreSnapshot;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AiError;

/// The expected schema version for snapshot bundle JSON files.
pub const SNAPSHOT_BUNDLE_VERSION: u32 = 1;

/// A self-contained JSON artifact representing a frozen moment in an NES game.
///
/// Because RL agents need to train on specific levels without playing the whole
/// game to get there, we use these bundles to instantly teleport the emulator
/// to the start of a training scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotBundle {
    /// The schema version of this bundle.
    pub version: u32,
    /// The SHA-256 hash of the ROM required to safely load this snapshot.
    pub rom_hash: String,
    /// A human-readable identifier (e.g. "smb-world-1-1").
    pub snapshot_id: String,
    /// The raw serialized memory state of the `NesCore`.
    pub snapshot: CoreSnapshot,
}

/// Writes a snapshot bundle containing the ROM hash, snapshot id, and emulator state.
///
/// # Errors
///
/// Returns [`AiError`] if the parent directory cannot be created, the bundle
/// cannot be serialized, or the file cannot be written.
pub fn write_snapshot_bundle(
    path: &Path,
    rom_hash: &str,
    snapshot_id: &str,
    snapshot: &CoreSnapshot,
) -> Result<(), AiError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|source| AiError::SnapshotDirCreate {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let bundle = SnapshotBundle {
        version: SNAPSHOT_BUNDLE_VERSION,
        rom_hash: rom_hash.to_owned(),
        snapshot_id: snapshot_id.to_owned(),
        snapshot: snapshot.clone(),
    };
    let json = serde_json::to_vec_pretty(&bundle)
        .map_err(|source| AiError::SnapshotSerialize { source })?;
    fs::write(path, json).map_err(|source| AiError::SnapshotWrite {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

/// Loads and validates a snapshot bundle from disk.
///
/// # Errors
///
/// Returns [`AiError`] if the file cannot be read, parsed, or if the bundle
/// version does not match [`SNAPSHOT_BUNDLE_VERSION`].
pub fn load_snapshot_bundle(path: &Path) -> Result<SnapshotBundle, AiError> {
    let bytes = fs::read(path).map_err(|source| AiError::SnapshotRead {
        path: path.to_path_buf(),
        source,
    })?;
    let bundle: SnapshotBundle =
        serde_json::from_slice(&bytes).map_err(|source| AiError::SnapshotParse {
            path: path.to_path_buf(),
            source,
        })?;
    if bundle.version != SNAPSHOT_BUNDLE_VERSION {
        return Err(AiError::SnapshotVersionMismatch {
            expected: SNAPSHOT_BUNDLE_VERSION,
            found: bundle.version,
        });
    }
    Ok(bundle)
}

/// Computes the SHA-256 hash of a byte slice and returns it as a lowercase hex string.
///
/// This is primarily used to fingerprint `.nes` ROM files to ensure that
/// loaded snapshots are not applied to the wrong game.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}
