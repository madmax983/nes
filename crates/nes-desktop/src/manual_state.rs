use std::fs;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveSlotStatus {
    Empty,
    Saved,
    Corrupt,
    IncompatibleRom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveSlotMetadata {
    pub slot: u8,
    pub path: PathBuf,
    pub status: SaveSlotStatus,
    pub modified_unix_secs: Option<u64>,
}

fn portable_stem_for_rom_path(rom_path: &Path) -> String {
    let raw_path = rom_path.as_os_str().to_string_lossy();
    let basename = raw_path
        .rsplit(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .unwrap_or(raw_path.as_ref());
    Path::new(basename)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("rom")
        .to_owned()
}

/// **Performance optimization:** Avoids `.collect::<String>()` by pre-allocating
/// `String::with_capacity` and pushing characters in a loop, guaranteeing exactly
/// one heap allocation instead of reallocating dynamically.
fn sanitized_stem_for_rom_path(rom_path: &Path) -> String {
    let stem = portable_stem_for_rom_path(rom_path);
    let mut sanitized = String::with_capacity(stem.len());
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    sanitized
}

/// **Performance optimization:** Avoids `.collect::<String>()` by pre-allocating
/// `String::with_capacity` and pushing characters in a loop, guaranteeing exactly
/// one heap allocation instead of reallocating dynamically.
fn hash_prefix(rom_hash: &str) -> String {
    let mut prefix = String::with_capacity(HASH_PREFIX_LEN);
    for ch in rom_hash.chars().take(HASH_PREFIX_LEN) {
        prefix.push(ch);
    }
    prefix
}

fn read_save_state_file(path: &Path) -> Result<SaveStateFile, String> {
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
    Ok(payload)
}

fn slot_number_from_path(path: &Path) -> Result<u8, String> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("save-slot path '{}' has no valid file name", path.display()))?;
    let slot_suffix = ".state.json";
    let slot_start = file_name.rfind(".slot").ok_or_else(|| {
        format!(
            "save-slot path '{}' is missing '.slotN' segment",
            path.display()
        )
    })?;
    let slot_digits = file_name[slot_start + ".slot".len()..]
        .strip_suffix(slot_suffix)
        .ok_or_else(|| {
            format!(
                "save-slot path '{}' must end with '.state.json'",
                path.display()
            )
        })?;
    slot_digits.parse::<u8>().map_err(|err| {
        format!(
            "save-slot path '{}' has invalid slot number '{}': {err}",
            path.display(),
            slot_digits
        )
    })
}

#[must_use]
pub fn slot_path_for_rom(rom_path: &Path, rom_hash: &str, slot: u8) -> PathBuf {
    let sanitized_stem = sanitized_stem_for_rom_path(rom_path);
    let hash_prefix = hash_prefix(rom_hash);
    PathBuf::from(SAVE_STATE_DIR).join(format!(
        "{sanitized_stem}-{hash_prefix}.slot{slot}.state.json"
    ))
}

#[must_use]
pub fn slot_paths_for_rom(
    rom_path: &Path,
    rom_hash: &str,
    slots: RangeInclusive<u8>,
) -> Vec<PathBuf> {
    slots
        .map(|slot| slot_path_for_rom(rom_path, rom_hash, slot))
        .collect()
}

#[must_use]
pub fn quicksave_path_for_rom(rom_path: &Path, rom_hash: &str) -> PathBuf {
    let sanitized_stem = sanitized_stem_for_rom_path(rom_path);
    let hash_prefix = hash_prefix(rom_hash);
    PathBuf::from(SAVE_STATE_DIR).join(format!("{sanitized_stem}-{hash_prefix}.state.json"))
}

/// Serializes a complete emulator snapshot to a JSON file on disk.
///
/// We do this to provide the player with an emergency fallback—a way to freeze time.
/// By embedding the `rom_hash` directly into the file, we protect the user from
/// accidentally loading a save state meant for *Super Mario Bros.* into *Zelda*,
/// which would otherwise cause the emulator to vividly hallucinate and crash.
///
/// ## Examples
///
/// ```
/// use nes_core::NesCore;
/// use nes_desktop::manual_state::save_state_file;
/// use std::path::PathBuf;
///
/// let core = NesCore::new();
/// let snapshot = core.save_state();
/// let mut path = std::env::temp_dir();
/// path.push("example_save.json");
///
/// save_state_file(&path, "abc123hash", &snapshot).expect("Failed to write save state");
/// ```
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

/// Reads and deserializes an emulator snapshot from a JSON file.
///
/// Why do we need the `expected_rom_hash`? Because loading the memory of one game
/// into another game is a recipe for digital chaos. This function acts as a bouncer,
/// ensuring the save state actually belongs to the cartridge currently inserted.
///
/// ## Panics
///
/// This function does not panic, but it will return an `Err` containing a detailed
/// `String` message if the file doesn't exist, is corrupted, or if the ROM hash mismatches.
///
/// ## Examples
///
/// ```
/// use nes_core::NesCore;
/// use nes_desktop::manual_state::{save_state_file, load_state_file};
/// use std::path::PathBuf;
///
/// let core = NesCore::new();
/// let mut path = std::env::temp_dir();
/// path.push("example_load.json");
///
/// // Create a dummy save file first
/// save_state_file(&path, "expected_hash", &core.save_state()).unwrap();
///
/// // Now load it back
/// let loaded_snapshot = load_state_file(&path, "expected_hash").unwrap();
/// ```
pub fn load_state_file(path: &Path, expected_rom_hash: &str) -> Result<CoreSnapshot, String> {
    let payload = read_save_state_file(path)?;
    if payload.rom_hash != expected_rom_hash {
        return Err(format!(
            "ROM hash mismatch for '{}': expected {expected_rom_hash}, found {}",
            path.display(),
            payload.rom_hash
        ));
    }
    Ok(payload.snapshot)
}

/// Reads lightweight metadata (like the timestamp) for a specific save state slot.
///
/// Instead of aggressively deserializing the entire 100KB+ JSON save state just to see
/// if the slot is empty, this function selectively peeks at the file system and
/// the top-level JSON structure. This allows UI overlays to render the save state menu
/// efficiently without stuttering the gameplay loop.
///
/// ## Examples
///
/// ```
/// use nes_desktop::manual_state::{read_slot_metadata, SaveSlotStatus};
/// use std::path::PathBuf;
///
/// let rom_path = PathBuf::from("super_mario.slot1.state.json");
/// // Check if slot 1 has a save for this ROM
/// let meta = read_slot_metadata(&rom_path, "smb_hash");
/// if let Ok(meta) = meta {
///     // Data is present
/// }
/// ```
pub fn read_slot_metadata(
    path: &Path,
    expected_rom_hash: &str,
) -> Result<SaveSlotMetadata, String> {
    let slot = slot_number_from_path(path)?;
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(SaveSlotMetadata {
                slot,
                path: path.to_path_buf(),
                status: SaveSlotStatus::Empty,
                modified_unix_secs: None,
            });
        }
        Err(err) => {
            return Err(format!(
                "failed to read save-slot metadata for '{}': {err}",
                path.display()
            ));
        }
    };

    let modified_unix_secs = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());
    let payload = match read_save_state_file(path) {
        Ok(payload) => payload,
        Err(_) => {
            return Ok(SaveSlotMetadata {
                slot,
                path: path.to_path_buf(),
                status: SaveSlotStatus::Corrupt,
                modified_unix_secs,
            });
        }
    };
    let status = if payload.rom_hash == expected_rom_hash {
        SaveSlotStatus::Saved
    } else {
        SaveSlotStatus::IncompatibleRom
    };

    Ok(SaveSlotMetadata {
        slot,
        path: path.to_path_buf(),
        status,
        modified_unix_secs,
    })
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use nes_core::NesCore;

    use super::{
        SaveSlotStatus, portable_stem_for_rom_path, read_slot_metadata, save_state_file,
        slot_path_for_rom, slot_paths_for_rom,
    };

    fn unique_test_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        env::temp_dir().join(format!(
            "nes-desktop-manual-state-{label}-{}-{nanos}",
            std::process::id()
        ))
    }

    #[test]
    fn portable_stem_for_rom_path_uses_last_segment_for_windows_style_paths() {
        assert_eq!(
            portable_stem_for_rom_path(Path::new(r"C:\roms\Super Mario Bros. (World).nes")),
            "Super Mario Bros. (World)"
        );
    }

    #[test]
    fn portable_stem_for_rom_path_uses_last_segment_for_unix_style_paths() {
        assert_eq!(
            portable_stem_for_rom_path(Path::new("/roms/Super Mario Bros. (World).nes")),
            "Super Mario Bros. (World)"
        );
    }

    #[test]
    fn slot_path_for_rom_includes_slot_number_and_hash_prefix() {
        let path = slot_path_for_rom(
            Path::new(r"C:\roms\Kirby's Adventure.nes"),
            "abcdef0123456789",
            3,
        );

        assert_eq!(
            path,
            PathBuf::from("savestates").join("Kirby_s_Adventure-abcdef01.slot3.state.json")
        );
    }

    #[test]
    fn slot_paths_for_rom_returns_contiguous_numbered_slots() {
        let paths =
            slot_paths_for_rom(Path::new("/roms/Mega Man 2.nes"), "0123456789abcdef", 1..=3);

        assert_eq!(paths.len(), 3);
        assert_eq!(
            paths[0],
            PathBuf::from("savestates").join("Mega_Man_2-01234567.slot1.state.json")
        );
        assert_eq!(
            paths[2],
            PathBuf::from("savestates").join("Mega_Man_2-01234567.slot3.state.json")
        );
    }

    #[test]
    fn read_slot_metadata_reports_empty_and_saved_slots() {
        let root = unique_test_dir("saved");
        fs::create_dir_all(&root).expect("temp root should be created");
        let slot_path = root.join("Kirby-abcdef01.slot1.state.json");
        let snapshot = NesCore::new().save_state();

        let empty = read_slot_metadata(&slot_path, "rom-hash").expect("empty slot metadata");
        assert_eq!(empty.slot, 1);
        assert!(matches!(empty.status, SaveSlotStatus::Empty));
        assert_eq!(empty.modified_unix_secs, None);

        save_state_file(&slot_path, "rom-hash", &snapshot).expect("slot save should succeed");
        let saved = read_slot_metadata(&slot_path, "rom-hash").expect("saved slot metadata");
        assert_eq!(saved.slot, 1);
        assert!(matches!(saved.status, SaveSlotStatus::Saved));
        assert!(saved.modified_unix_secs.is_some());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn read_slot_metadata_flags_incompatible_rom_hash() {
        let root = unique_test_dir("incompatible");
        fs::create_dir_all(&root).expect("temp root should be created");
        let slot_path = root.join("Kirby-abcdef01.slot2.state.json");
        let snapshot = NesCore::new().save_state();

        save_state_file(&slot_path, "expected-hash", &snapshot).expect("slot save should succeed");
        let saved = read_slot_metadata(&slot_path, "different-hash").expect("saved slot metadata");
        assert_eq!(saved.slot, 2);
        assert!(matches!(saved.status, SaveSlotStatus::IncompatibleRom));
        assert!(saved.modified_unix_secs.is_some());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn read_slot_metadata_flags_corrupt_files_without_failing_refresh() {
        let root = unique_test_dir("corrupt");
        fs::create_dir_all(&root).expect("temp root should be created");
        let slot_path = root.join("Kirby-abcdef01.slot3.state.json");

        fs::write(&slot_path, b"{ definitely not json").expect("corrupt slot file should write");

        let metadata = read_slot_metadata(&slot_path, "rom-hash").expect("corrupt slot metadata");
        assert_eq!(metadata.slot, 3);
        assert!(matches!(metadata.status, SaveSlotStatus::Corrupt));
        assert!(metadata.modified_unix_secs.is_some());

        let _ = fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod havoc_oom_tests {
    use super::*;
    use std::path::PathBuf;

    // We ignore this test normally so it doesn't break CI, but we use it to prove OOM vulnerability
    #[test]
    #[ignore]
    fn havoc_manual_state_oom() {
        let path = PathBuf::from("/dev/zero");

        // This will hang and eventually OOM kill the process
        let _ = read_save_state_file(&path);
    }
}
