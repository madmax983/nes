use std::path::Path;
use std::{fmt::Write as _, fs};

use nes_core::CoreSnapshot;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AiError;

/// The expected version of the snapshot bundle schema.
pub const SNAPSHOT_BUNDLE_VERSION: u32 = 1;

/// A serialized representation of a full emulator snapshot along with its metadata.
///
/// Includes the `CoreSnapshot` itself, as well as the hash of the ROM that it
/// requires in order to run correctly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotBundle {
    /// The version of the snapshot schema.
    pub version: u32,
    /// The SHA256 hash of the ROM file used to generate this snapshot.
    pub rom_hash: String,
    /// A human-readable identifier for this snapshot.
    pub snapshot_id: String,
    /// The actual serialized emulator core state.
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

/// Computes the SHA256 hash of a byte slice and returns it as a lowercase hex string.
///
/// ## Examples
/// ```
/// use nes_ai::snapshot::sha256_hex;
/// let hash = sha256_hex(b"hello world");
/// assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
/// ```
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}
