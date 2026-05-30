/// Shared high-level intents emitted by the native menu, overlay, and hotkeys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    /// Toggles the visibility of the UI overlay.
    ToggleOverlay,
    /// Resumes the emulation if it is currently paused.
    Resume,
    /// Opens a file dialog to load a new ROM.
    OpenRom,
    /// Opens the cheat management menu.
    OpenCheats,
    /// Saves the current state to the specified slot.
    SaveSlot(u8),
    /// Loads the state from the specified slot.
    LoadSlot(u8),
    /// Resets the current emulation session.
    Reset,
    /// Exits the application entirely.
    Quit,
}

/// Returns the stable string id used for menu item routing.
#[must_use]
pub fn menu_id_for_action(action: AppAction) -> String {
    match action {
        AppAction::ToggleOverlay => "overlay.toggle".to_owned(),
        AppAction::Resume => "emulation.resume".to_owned(),
        AppAction::OpenRom => "file.open_rom".to_owned(),
        AppAction::OpenCheats => "emulation.cheats".to_owned(),
        AppAction::SaveSlot(slot) => format!("state.save.{slot}"),
        AppAction::LoadSlot(slot) => format!("state.load.{slot}"),
        AppAction::Reset => "emulation.reset".to_owned(),
        AppAction::Quit => "file.quit".to_owned(),
    }
}

/// Parses a stable menu item id back into an [`AppAction`].
#[must_use]
pub fn action_from_menu_id(id: &str) -> Option<AppAction> {
    match id {
        "overlay.toggle" => Some(AppAction::ToggleOverlay),
        "emulation.resume" => Some(AppAction::Resume),
        "file.open_rom" => Some(AppAction::OpenRom),
        "emulation.cheats" => Some(AppAction::OpenCheats),
        "emulation.reset" => Some(AppAction::Reset),
        "file.quit" => Some(AppAction::Quit),
        _ => {
            if let Some(slot) = id.strip_prefix("state.save.") {
                return slot.parse().ok().map(AppAction::SaveSlot);
            }
            if let Some(slot) = id.strip_prefix("state.load.") {
                return slot.parse().ok().map(AppAction::LoadSlot);
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppAction, action_from_menu_id, menu_id_for_action};

    #[test]
    fn menu_ids_roundtrip_common_actions_and_slots() {
        let actions = [
            AppAction::ToggleOverlay,
            AppAction::Resume,
            AppAction::OpenRom,
            AppAction::OpenCheats,
            AppAction::SaveSlot(1),
            AppAction::SaveSlot(5),
            AppAction::LoadSlot(1),
            AppAction::LoadSlot(5),
            AppAction::Reset,
            AppAction::Quit,
        ];

        for action in actions {
            let id = menu_id_for_action(action);
            assert_eq!(action_from_menu_id(&id), Some(action));
        }
    }
}
