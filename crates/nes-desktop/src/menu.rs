use std::path::PathBuf;

#[cfg(all(not(test), any(target_os = "windows", target_os = "macos")))]
use std::collections::BTreeMap;

#[cfg(all(not(test), any(target_os = "windows", target_os = "macos")))]
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

#[cfg(all(not(test), target_os = "windows"))]
use winit::platform::windows::WindowExtWindows;
use winit::window::Window;

use crate::actions::{AppAction, action_from_menu_id, menu_id_for_action};

/// Pure description of the desktop menu tree.
#[cfg_attr(test, derive(Debug, Clone, PartialEq, Eq))]
pub struct DesktopMenu {
    entries: Vec<DesktopMenuEntry>,
    #[cfg(all(not(test), any(target_os = "windows", target_os = "macos")))]
    native: NativeDesktopMenu,
}

impl DesktopMenu {
    #[must_use]
    /// Returns the menu entries.
    pub fn entries(&self) -> &[DesktopMenuEntry] {
        &self.entries
    }

    #[cfg(not(test))]
    /// Installs the menu to the given window.
    pub fn install_for_window(&self, window: &Window) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        unsafe {
            self.native
                .root
                .init_for_hwnd(window.hwnd())
                .map_err(|err| format!("Failed to attach native menu: {err}"))?;
        }

        #[cfg(target_os = "macos")]
        self.native.root.init_for_nsapp();

        #[cfg(target_os = "linux")]
        let _ = window;

        Ok(())
    }

    #[cfg(not(test))]
    #[must_use]
    /// Polls for a menu action event.
    pub fn poll_action(&self) -> Option<AppAction> {
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            while let Ok(event) = MenuEvent::receiver().try_recv() {
                if let Some(action) = action_from_menu_event_id(event.id.as_ref()) {
                    return Some(action);
                }
            }
        }
        None
    }

    #[cfg(not(test))]
    /// Enables or disables a specific menu action.
    pub fn set_action_enabled(&self, action: AppAction, enabled: bool) {
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            let id = menu_id_for_action(action);
            if let Some(item) = self.native.items.get(&id) {
                item.set_enabled(enabled);
            }
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let _ = (action, enabled);
    }

    #[cfg(not(test))]
    /// Syncs the menu state with the runtime emulator state.
    pub fn sync_runtime_state(&self, rollback_enabled: bool) {
        sync_runtime_entries(self, &self.entries, rollback_enabled);
    }

    #[cfg(test)]
    /// Installs the menu to the given window.
    pub fn install_for_window(&self, window: &Window) -> Result<(), String> {
        let _ = window;
        Ok(())
    }

    #[cfg(test)]
    #[must_use]
    /// Polls for a menu action event.
    pub fn poll_action(&self) -> Option<AppAction> {
        None
    }

    #[cfg(test)]
    /// Enables or disables a specific menu action.
    pub fn set_action_enabled(&self, action: AppAction, enabled: bool) {
        let _ = (action, enabled);
    }
}

/// Declarative menu node used to keep menu semantics testable.
#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents an entry in the desktop menu.
pub enum DesktopMenuEntry {
    /// A selectable item.
    Item(MenuItemSpec),
    /// A visual separator.
    Separator,
    /// A nested submenu.
    Submenu {
        /// The label of the submenu.
        label: &'static str,
        /// The entries within the submenu.
        entries: Vec<DesktopMenuEntry>,
    },
}

/// Shared metadata for actionable menu items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItemSpec {
    /// Unique ID for the action.
    pub id: String,
    /// Label for the action.
    pub label: String,
}

#[cfg(all(not(test), any(target_os = "windows", target_os = "macos")))]
struct NativeDesktopMenu {
    root: Menu,
    items: BTreeMap<String, MenuItem>,
}

/// Builds the menu tree used by the native menu adapter.
#[must_use]
pub fn build_native_menu(slot_count: u8) -> DesktopMenu {
    let file_entries = vec![
        DesktopMenuEntry::Item(item(AppAction::OpenRom, "Open ROM...")),
        DesktopMenuEntry::Separator,
        DesktopMenuEntry::Item(item(AppAction::Quit, "Quit")),
    ];

    let mut emulation_entries = vec![DesktopMenuEntry::Item(item(AppAction::Resume, "Resume"))];
    emulation_entries.push(DesktopMenuEntry::Item(item(
        AppAction::OpenCheats,
        "Cheats...",
    )));
    emulation_entries.extend(slot_entries(AppAction::SaveSlot, "Save Slot", slot_count));
    emulation_entries.extend(slot_entries(AppAction::LoadSlot, "Load Slot", slot_count));
    emulation_entries.push(DesktopMenuEntry::Separator);
    emulation_entries.push(DesktopMenuEntry::Item(item(AppAction::Reset, "Reset")));

    let entries = vec![
        DesktopMenuEntry::Submenu {
            label: "File",
            entries: file_entries,
        },
        DesktopMenuEntry::Submenu {
            label: "Emulation",
            entries: emulation_entries,
        },
    ];

    #[cfg(all(not(test), any(target_os = "windows", target_os = "macos")))]
    let native = build_native_adapter(&entries).expect("desktop menu tree should be valid");

    DesktopMenu {
        entries,
        #[cfg(all(not(test), any(target_os = "windows", target_os = "macos")))]
        native,
    }
}

/// Converts a menu event id into its matching [`AppAction`].
#[must_use]
pub fn action_from_menu_event_id(id: &str) -> Option<AppAction> {
    action_from_menu_id(id)
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
#[must_use]
/// Checks if native menus are supported on this platform.
pub const fn native_menu_supported() -> bool {
    true
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[must_use]
/// Checks if native menus are supported on this platform.
pub const fn native_menu_supported() -> bool {
    false
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
#[must_use]
/// Checks if a native file picker is supported on this platform.
pub const fn rom_picker_supported() -> bool {
    true
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[must_use]
/// Checks if a native file picker is supported on this platform.
pub const fn rom_picker_supported() -> bool {
    false
}

/// Returns whether the action should be enabled for the current runtime mode.
#[must_use]
pub fn action_is_enabled_for_runtime(action: AppAction, rollback_enabled: bool) -> bool {
    !(rollback_enabled
        && matches!(
            action,
            AppAction::OpenRom
                | AppAction::OpenCheats
                | AppAction::SaveSlot(_)
                | AppAction::LoadSlot(_)
        ))
}

/// Opens the native ROM picker in production builds.
#[cfg(all(not(test), any(target_os = "windows", target_os = "macos")))]
pub fn pick_rom_path() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("NES ROM", &["nes"])
        .pick_file()
}

/// Returns no file on targets without a native picker integration in this build.
#[cfg(any(test, not(any(target_os = "windows", target_os = "macos"))))]
pub fn pick_rom_path() -> Option<PathBuf> {
    None
}

fn item(action: AppAction, label: &str) -> MenuItemSpec {
    MenuItemSpec {
        id: menu_id_for_action(action),
        label: label.to_owned(),
    }
}

fn slot_entries(ctor: fn(u8) -> AppAction, prefix: &str, slot_count: u8) -> Vec<DesktopMenuEntry> {
    (1..=slot_count)
        .map(|slot| DesktopMenuEntry::Item(item(ctor(slot), &format!("{prefix} {slot}"))))
        .collect()
}

#[cfg(all(not(test), any(target_os = "windows", target_os = "macos")))]
fn build_native_adapter(entries: &[DesktopMenuEntry]) -> Result<NativeDesktopMenu, String> {
    let root = Menu::new();
    let mut items = BTreeMap::new();
    for entry in entries {
        let DesktopMenuEntry::Submenu { label, entries } = entry else {
            return Err("desktop menu root entries must be submenus".to_owned());
        };

        let submenu = Submenu::new(*label, true);
        append_submenu_entries(&submenu, entries, &mut items)?;
        root.append(&submenu)
            .map_err(|err| format!("Failed to append submenu `{label}`: {err}"))?;
    }

    Ok(NativeDesktopMenu { root, items })
}

#[cfg(all(not(test), any(target_os = "windows", target_os = "macos")))]
fn append_submenu_entries(
    submenu: &Submenu,
    entries: &[DesktopMenuEntry],
    items: &mut BTreeMap<String, MenuItem>,
) -> Result<(), String> {
    for entry in entries {
        match entry {
            DesktopMenuEntry::Item(spec) => {
                let item = MenuItem::with_id(&spec.id, &spec.label, true, None);
                submenu
                    .append(&item)
                    .map_err(|err| format!("Failed to append menu item `{}`: {err}", spec.label))?;
                items.insert(spec.id.clone(), item);
            }
            DesktopMenuEntry::Separator => {
                let separator = PredefinedMenuItem::separator();
                submenu
                    .append(&separator)
                    .map_err(|err| format!("Failed to append separator: {err}"))?;
            }
            DesktopMenuEntry::Submenu { label, entries } => {
                let child = Submenu::new(*label, true);
                append_submenu_entries(&child, entries, items)?;
                submenu
                    .append(&child)
                    .map_err(|err| format!("Failed to append submenu `{label}`: {err}"))?;
            }
        }
    }

    Ok(())
}

#[cfg(not(test))]
fn sync_runtime_entries(menu: &DesktopMenu, entries: &[DesktopMenuEntry], rollback_enabled: bool) {
    for entry in entries {
        match entry {
            DesktopMenuEntry::Item(spec) => {
                if let Some(action) = action_from_menu_event_id(&spec.id) {
                    menu.set_action_enabled(
                        action,
                        action_is_enabled_for_runtime(action, rollback_enabled),
                    );
                }
            }
            DesktopMenuEntry::Separator => {}
            DesktopMenuEntry::Submenu { entries, .. } => {
                sync_runtime_entries(menu, entries, rollback_enabled);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppAction, DesktopMenuEntry, action_from_menu_event_id, action_is_enabled_for_runtime,
        build_native_menu, rom_picker_supported,
    };

    #[test]
    fn menu_event_ids_map_to_expected_actions() {
        assert_eq!(
            action_from_menu_event_id("file.open_rom"),
            Some(AppAction::OpenRom)
        );
        assert_eq!(
            action_from_menu_event_id("state.save.4"),
            Some(AppAction::SaveSlot(4))
        );
        assert_eq!(
            action_from_menu_event_id("state.load.2"),
            Some(AppAction::LoadSlot(2))
        );
        assert_eq!(
            action_from_menu_event_id("emulation.cheats"),
            Some(AppAction::OpenCheats)
        );
        assert_eq!(
            action_from_menu_event_id("file.quit"),
            Some(AppAction::Quit)
        );
    }

    #[test]
    fn native_menu_contains_file_and_emulation_sections() {
        let menu = build_native_menu(3);

        assert_eq!(menu.entries().len(), 2);
        assert!(matches!(
            &menu.entries()[0],
            DesktopMenuEntry::Submenu { label, .. } if *label == "File"
        ));
        assert!(matches!(
            &menu.entries()[1],
            DesktopMenuEntry::Submenu { label, .. } if *label == "Emulation"
        ));
    }

    #[test]
    fn native_menu_emulation_section_contains_cheats_entry() {
        let menu = build_native_menu(2);
        let DesktopMenuEntry::Submenu { entries, .. } = &menu.entries()[1] else {
            panic!("expected emulation submenu");
        };

        assert!(entries.iter().any(|entry| {
            matches!(
                entry,
                DesktopMenuEntry::Item(spec)
                    if spec.id == "emulation.cheats" && spec.label == "Cheats..."
            )
        }));
    }

    #[test]
    fn rollback_disables_open_and_slot_actions() {
        assert!(!action_is_enabled_for_runtime(AppAction::OpenRom, true));
        assert!(!action_is_enabled_for_runtime(AppAction::OpenCheats, true));
        assert!(!action_is_enabled_for_runtime(AppAction::SaveSlot(2), true));
        assert!(!action_is_enabled_for_runtime(AppAction::LoadSlot(4), true));
        assert!(action_is_enabled_for_runtime(AppAction::Reset, true));
        assert!(action_is_enabled_for_runtime(AppAction::Quit, true));
    }

    #[test]
    fn rom_picker_capability_matches_platform_contract() {
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        assert!(rom_picker_supported());

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            assert!(!rom_picker_supported());
            assert!(super::pick_rom_path().is_none());
        }
    }
}
