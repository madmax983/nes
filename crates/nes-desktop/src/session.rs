use crossterm::style::{Color, Stylize};
use nes_core::{NesCore, RomLoadInfo};
use std::fs;
use std::path::{Path, PathBuf};

use nes_desktop::manual_state::{
    SaveSlotMetadata, SaveSlotStatus, read_slot_metadata, slot_path_for_rom, slot_paths_for_rom,
};
use nes_desktop::overlay::OverlaySlotSummary;
use nes_desktop::rta::compute_rom_hash;
use nes_desktop::session_cheats::SessionCheats;

// Assuming SAVE_SLOT_COUNT is 5 based on main.rs
pub const SAVE_SLOT_COUNT: u8 = 5;

pub(crate) struct LoadedRomSession {
    pub(crate) rom_path: PathBuf,
    pub(crate) rom_hash: String,
    pub(crate) info: RomLoadInfo,
    pub(crate) slot_metadata: Vec<SaveSlotMetadata>,
}

pub(crate) fn apply_runtime_cheat_codes(
    core: &mut NesCore,
    cheat_codes: &[String],
) -> Result<(), String> {
    core.clear_cheat_codes();
    for raw_code in cheat_codes {
        core.add_cheat_code(raw_code)
            .map_err(|err| format!("Invalid cheat code '{raw_code}': {err}"))?;
    }
    Ok(())
}

pub(crate) fn apply_session_cheats(
    core: &mut NesCore,
    cheats: &SessionCheats,
) -> Result<(), String> {
    apply_runtime_cheat_codes(core, &cheats.enabled_codes())
}

pub(crate) fn load_rom_session(
    core: &mut NesCore,
    rom_path: &Path,
    cheats: &SessionCheats,
) -> Result<LoadedRomSession, String> {
    let rom_bytes = fs::read(rom_path)
        .map_err(|err| format_rom_read_error(&rom_path.display().to_string(), &err))?;
    core.clear_cheat_codes();
    let info = core
        .load_ines_rom(&rom_bytes)
        .map_err(|err| format!("Failed to load ROM: {err}"))?;
    apply_session_cheats(core, cheats)?;
    let rom_hash = compute_rom_hash(&rom_bytes);
    let slot_metadata = load_slot_metadata_for_rom(rom_path, &rom_hash)?;
    Ok(LoadedRomSession {
        rom_path: rom_path.to_path_buf(),
        rom_hash,
        info,
        slot_metadata,
    })
}

pub(crate) fn load_slot_metadata_for_rom(
    rom_path: &Path,
    rom_hash: &str,
) -> Result<Vec<SaveSlotMetadata>, String> {
    slot_paths_for_rom(rom_path, rom_hash, 1..=SAVE_SLOT_COUNT)
        .into_iter()
        .map(|path| read_slot_metadata(&path, rom_hash))
        .collect()
}

pub(crate) fn refresh_slot_metadata(session: &mut LoadedRomSession) -> Result<(), String> {
    session.slot_metadata = load_slot_metadata_for_rom(&session.rom_path, &session.rom_hash)?;
    Ok(())
}

pub(crate) fn slot_path_for_selection(session: &LoadedRomSession, slot: u8) -> PathBuf {
    slot_path_for_rom(&session.rom_path, &session.rom_hash, slot)
}

pub(crate) fn format_slot_status(metadata: &SaveSlotMetadata) -> OverlaySlotSummary {
    let status_label = match metadata.status {
        SaveSlotStatus::Empty => "Empty",
        SaveSlotStatus::Saved => "Saved",
        SaveSlotStatus::Corrupt => "Corrupt",
        SaveSlotStatus::IncompatibleRom => "Mismatch",
    };
    OverlaySlotSummary {
        slot: metadata.slot,
        status_label,
        modified_unix_secs: metadata.modified_unix_secs,
    }
}

pub(crate) fn rom_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("ROM")
        .to_owned()
}

pub(crate) fn window_title(session: &LoadedRomSession, overlay_open: bool) -> String {
    let suffix = if overlay_open { " [Paused]" } else { "" };
    format!(
        "nes-desktop - {}{suffix}",
        rom_display_name(&session.rom_path)
    )
}

pub(crate) fn format_rom_read_error(rom_path: &str, err: &std::io::Error) -> String {
    if err.kind() == std::io::ErrorKind::NotFound {
        format!(
            "{} Could not find the ROM file at '{}'.\n{} Check the path or try the bundled homebrew ROM: ./roms/homebrew/homebrew.nes",
            "Error:".with(Color::Red).bold(),
            rom_path.with(Color::Yellow),
            "Hint:".with(Color::Cyan).bold()
        )
    } else {
        format!(
            "{} Failed to read ROM at '{}': {}",
            "Error:".with(Color::Red).bold(),
            rom_path,
            err
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nes_core::NesCore;

    #[test]
    fn applying_runtime_cheat_codes_replaces_existing_codes() {
        let mut core = NesCore::new();
        apply_runtime_cheat_codes(&mut core, &[String::from("GOSSIP")])
            .expect("first cheat application should succeed");
        apply_runtime_cheat_codes(&mut core, &[String::from("GOSSIP")])
            .expect("second cheat application should succeed");
        assert_eq!(core.cheat_codes().len(), 1);
    }
}
