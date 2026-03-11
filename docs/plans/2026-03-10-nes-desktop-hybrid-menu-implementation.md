# NES Desktop Hybrid Menu Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a cross-platform hybrid menu to `nes-desktop` with a native menu bar, modal in-window pause overlay, native ROM picker, and per-ROM numbered save slots.

**Architecture:** Keep the current `winit + pixels` desktop loop and add a thin platform adapter layer for native menu/file-picker integrations. All user intents flow through one shared `AppAction` command layer so hotkeys, native menu clicks, and overlay selections all execute the same runtime path. The overlay is rendered directly into the existing RGBA framebuffer with a tiny bitmap text renderer rather than introducing a full UI framework.

**Tech Stack:** Rust 2024, `winit`, `pixels`, `muda`, `rfd`, `font8x8`, `serde_json`, `nes-core`

---

## Validated v1 Decisions

- Hybrid UI: native menu bar plus in-window overlay
- Cross-platform immediately
- `Open ROM` uses a native file picker
- Overlay is modal and pauses emulation while open
- Save states use numbered slots (`1..=5`) per ROM

## v1 Assumptions To Implement

- `Escape` toggles the modal overlay instead of quitting immediately
- `F5` saves to the currently selected slot; `F8` loads from the currently selected slot
- Selected slot defaults to slot `1` and can be changed from the overlay
- `Open ROM`, `Save Slot`, and `Load Slot` are disabled while rollback/netplay is active
- Overlay rendering uses `font8x8` bitmap glyphs and simple filled rectangles

---

## Task 1: Add Shared Action Types And Cross-Platform UI Dependencies

**Files:**
- Modify: `crates/nes-desktop/Cargo.toml`
- Modify: `crates/nes-desktop/src/lib.rs`
- Create: `crates/nes-desktop/src/actions.rs`

**Step 1: Write the failing test**

In `crates/nes-desktop/src/actions.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::{AppAction, action_from_menu_id, menu_id_for_action};

    #[test]
    fn menu_ids_roundtrip_common_actions_and_slots() {
        let actions = [
            AppAction::ToggleOverlay,
            AppAction::Resume,
            AppAction::OpenRom,
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
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p nes-desktop menu_ids_roundtrip_common_actions_and_slots --lib
```

Expected: FAIL because `actions.rs` and the shared action helpers do not exist yet.

**Step 3: Write minimal implementation**

- Add these dependencies to `crates/nes-desktop/Cargo.toml`:

```toml
muda = "0.15"
rfd = "0.15"
font8x8 = "0.3"
```

- Create `crates/nes-desktop/src/actions.rs` with:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    ToggleOverlay,
    Resume,
    OpenRom,
    SaveSlot(u8),
    LoadSlot(u8),
    Reset,
    Quit,
}

pub fn menu_id_for_action(action: AppAction) -> String {
    match action {
        AppAction::ToggleOverlay => "overlay.toggle".to_owned(),
        AppAction::Resume => "emulation.resume".to_owned(),
        AppAction::OpenRom => "file.open_rom".to_owned(),
        AppAction::SaveSlot(slot) => format!("state.save.{slot}"),
        AppAction::LoadSlot(slot) => format!("state.load.{slot}"),
        AppAction::Reset => "emulation.reset".to_owned(),
        AppAction::Quit => "file.quit".to_owned(),
    }
}

pub fn action_from_menu_id(id: &str) -> Option<AppAction> {
    match id {
        "overlay.toggle" => Some(AppAction::ToggleOverlay),
        "emulation.resume" => Some(AppAction::Resume),
        "file.open_rom" => Some(AppAction::OpenRom),
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
```

- Export the module from `crates/nes-desktop/src/lib.rs`:

```rust
pub mod actions;
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p nes-desktop menu_ids_roundtrip_common_actions_and_slots --lib
```

Expected: PASS

**Step 5: Commit**

```bash
git add crates/nes-desktop/Cargo.toml crates/nes-desktop/src/lib.rs crates/nes-desktop/src/actions.rs
git commit -m "feat: add shared desktop action ids for menu and overlay"
```

---

## Task 2: Upgrade Manual Save-State Support To Numbered Per-ROM Slots

**Files:**
- Modify: `crates/nes-desktop/src/manual_state.rs`

**Step 1: Write the failing tests**

In `crates/nes-desktop/src/manual_state.rs`, add:

```rust
#[test]
fn slot_path_for_rom_includes_slot_number_and_hash_prefix() {
    let path = slot_path_for_rom(
        Path::new(r"C:\roms\Kirby's Adventure.nes"),
        "abcdef0123456789",
        3,
    );

    assert_eq!(path, PathBuf::from("savestates").join("Kirby_s_Adventure-abcdef01.slot3.state.json"));
}

#[test]
fn load_slot_metadata_reports_empty_and_saved_slots() {
    let temp = tempfile::tempdir().expect("temp dir");
    let slot_path = temp.path().join("slot1.state.json");
    let snapshot = nes_core::NesCore::new().save_state();

    let empty = read_slot_metadata(&slot_path, "rom-hash").expect("empty metadata");
    assert!(matches!(empty.status, SaveSlotStatus::Empty));

    save_state_file(&slot_path, "rom-hash", &snapshot).expect("save slot");
    let saved = read_slot_metadata(&slot_path, "rom-hash").expect("saved metadata");
    assert!(matches!(saved.status, SaveSlotStatus::Saved));
    assert_eq!(saved.slot, 1);
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -p nes-desktop slot_path_for_rom_includes_slot_number_and_hash_prefix --lib
cargo test -p nes-desktop load_slot_metadata_reports_empty_and_saved_slots --lib
```

Expected: FAIL because slot helpers and slot metadata types do not exist.

**Step 3: Write minimal implementation**

- Add `tempfile = "3"` to `[dev-dependencies]` in `crates/nes-desktop/Cargo.toml` if it is not already present.
- In `crates/nes-desktop/src/manual_state.rs`:
  - replace `quicksave_path_for_rom` with `slot_path_for_rom(rom_path, rom_hash, slot)`
  - add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveSlotStatus {
    Empty,
    Saved,
    IncompatibleRom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveSlotMetadata {
    pub slot: u8,
    pub path: PathBuf,
    pub status: SaveSlotStatus,
    pub modified_unix_secs: Option<u64>,
}
```

  - implement `read_slot_metadata(path, expected_rom_hash) -> Result<SaveSlotMetadata, String>`
  - implement `slot_paths_for_rom(rom_path, rom_hash, slots: RangeInclusive<u8>) -> Vec<PathBuf>`
  - keep `save_state_file` / `load_state_file`, but use them through slot paths

**Step 4: Run tests to verify they pass**

```bash
cargo test -p nes-desktop slot_path_for_rom_includes_slot_number_and_hash_prefix --lib
cargo test -p nes-desktop load_slot_metadata_reports_empty_and_saved_slots --lib
```

Expected: PASS

**Step 5: Commit**

```bash
git add crates/nes-desktop/Cargo.toml crates/nes-desktop/src/manual_state.rs
git commit -m "feat: add numbered per-rom save slot metadata"
```

---

## Task 3: Add Modal Overlay State And Bitmap Renderer

**Files:**
- Create: `crates/nes-desktop/src/overlay.rs`
- Modify: `crates/nes-desktop/src/lib.rs`

**Step 1: Write the failing tests**

In `crates/nes-desktop/src/overlay.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::{OverlayEntry, OverlayModel, OverlaySelection, draw_text};

    #[test]
    fn overlay_navigation_wraps_and_tracks_selected_slot() {
        let mut overlay = OverlayModel::new(3);
        assert_eq!(overlay.selection(), OverlaySelection::Resume);

        overlay.move_prev();
        assert_eq!(overlay.selection(), OverlaySelection::Quit);

        overlay.move_next();
        overlay.move_next();
        assert_eq!(overlay.selection(), OverlaySelection::SaveSlot(1));

        overlay.activate();
        assert_eq!(overlay.selected_slot(), 1);
    }

    #[test]
    fn draw_text_marks_pixels_inside_target_buffer() {
        let mut frame = vec![0_u8; 64 * 64 * 4];
        draw_text(&mut frame, 64, 2, 2, "NES", [255, 255, 255, 255]);
        assert!(frame.iter().any(|component| *component != 0));
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -p nes-desktop overlay_navigation_wraps_and_tracks_selected_slot --lib
cargo test -p nes-desktop draw_text_marks_pixels_inside_target_buffer --lib
```

Expected: FAIL because the overlay model and draw helpers do not exist.

**Step 3: Write minimal implementation**

- Create `crates/nes-desktop/src/overlay.rs` with:
  - `OverlaySelection` enum:
    - `Resume`
    - `OpenRom`
    - `SaveSlot(u8)`
    - `LoadSlot(u8)`
    - `Reset`
    - `Quit`
  - `OverlayModel` struct:
    - `open: bool`
    - `selected_slot: u8`
    - `selection_index: usize`
    - `entries: Vec<OverlayEntry>`
    - `status_message: Option<String>`
  - methods:
    - `new(slot_count: u8) -> Self`
    - `open()`, `close()`, `toggle()`
    - `move_prev()`, `move_next()`
    - `selection() -> OverlaySelection`
    - `activate() -> OverlaySelection`
    - `set_status_message(...)`
  - rendering helpers:
    - `fill_rect(...)`
    - `draw_text(...)` using `font8x8`
    - `draw_overlay(...)`

- Export from `crates/nes-desktop/src/lib.rs`:

```rust
pub mod overlay;
```

**Step 4: Run tests to verify they pass**

```bash
cargo test -p nes-desktop overlay_navigation_wraps_and_tracks_selected_slot --lib
cargo test -p nes-desktop draw_text_marks_pixels_inside_target_buffer --lib
```

Expected: PASS

**Step 5: Commit**

```bash
git add crates/nes-desktop/src/lib.rs crates/nes-desktop/src/overlay.rs
git commit -m "feat: add modal pause overlay model and bitmap renderer"
```

---

## Task 4: Add Native Menu And Cross-Platform File Picker Adapters

**Files:**
- Create: `crates/nes-desktop/src/menu.rs`
- Modify: `crates/nes-desktop/src/lib.rs`

**Step 1: Write the failing test**

In `crates/nes-desktop/src/menu.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use crate::actions::AppAction;
    use super::action_from_menu_event_id;

    #[test]
    fn menu_event_ids_map_to_expected_actions() {
        assert_eq!(action_from_menu_event_id("file.open_rom"), Some(AppAction::OpenRom));
        assert_eq!(action_from_menu_event_id("state.save.4"), Some(AppAction::SaveSlot(4)));
        assert_eq!(action_from_menu_event_id("state.load.2"), Some(AppAction::LoadSlot(2)));
        assert_eq!(action_from_menu_event_id("file.quit"), Some(AppAction::Quit));
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p nes-desktop menu_event_ids_map_to_expected_actions --lib
```

Expected: FAIL because `menu.rs` does not exist.

**Step 3: Write minimal implementation**

- Create `crates/nes-desktop/src/menu.rs` with:
  - `DesktopMenu` wrapper holding the native `muda` menu tree
  - `build_native_menu(slot_count: u8) -> DesktopMenu`
  - `action_from_menu_event_id(id: &str) -> Option<AppAction>` delegating to `actions.rs`
  - `pick_rom_path() -> Option<PathBuf>` using:

```rust
rfd::FileDialog::new()
    .add_filter("NES ROM", &["nes"])
    .pick_file()
```

  - string IDs:
    - `file.open_rom`
    - `file.quit`
    - `emulation.resume`
    - `emulation.reset`
    - `state.save.<slot>`
    - `state.load.<slot>`

- Export from `crates/nes-desktop/src/lib.rs`:

```rust
pub mod menu;
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p nes-desktop menu_event_ids_map_to_expected_actions --lib
```

Expected: PASS

**Step 5: Commit**

```bash
git add crates/nes-desktop/src/lib.rs crates/nes-desktop/src/menu.rs
git commit -m "feat: add native desktop menu and file picker adapters"
```

---

## Task 5: Extract Runtime Helpers For ROM Sessions And Action Execution

**Files:**
- Modify: `crates/nes-desktop/src/main.rs`

**Step 1: Write the failing tests**

In `crates/nes-desktop/src/main.rs` tests, add:

```rust
#[test]
fn selected_slot_hotkeys_target_current_slot() {
    assert_eq!(slot_action_for_hotkey(true, 3), Some(AppAction::SaveSlot(3)));
    assert_eq!(slot_action_for_hotkey(false, 3), Some(AppAction::LoadSlot(3)));
}

#[test]
fn overlay_escape_toggles_pause_menu_instead_of_exit() {
    assert_eq!(
        classify_keyboard_input(VirtualKeyCode::Escape, true, false, false, false),
        KeyboardDecision::ToggleOverlay
    );
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -p nes-desktop selected_slot_hotkeys_target_current_slot --bin nes-desktop
cargo test -p nes-desktop overlay_escape_toggles_pause_menu_instead_of_exit --bin nes-desktop
```

Expected: FAIL because `AppAction` is not wired into `main.rs` and `Escape` still exits.

**Step 3: Write minimal implementation**

- In `crates/nes-desktop/src/main.rs`:
  - import `AppAction`, `DesktopMenu`, `pick_rom_path`, `OverlayModel`, and slot metadata helpers
  - add:

```rust
struct LoadedRomSession {
    rom_path: PathBuf,
    rom_hash: String,
    slot_metadata: Vec<SaveSlotMetadata>,
    selected_slot: u8,
}
```

  - extract ROM/session helpers:
    - `load_rom_session(core, rom_path, cheat_codes) -> Result<LoadedRomSession, String>`
    - `refresh_slot_metadata(session) -> Result<(), String>`
    - `save_selected_slot(...)`
    - `load_selected_slot(...)`
  - add `slot_action_for_hotkey(is_save: bool, selected_slot: u8) -> Option<AppAction>`
  - update `KeyboardDecision` to include `ToggleOverlay`
  - change `classify_keyboard_input` so `Escape` returns `ToggleOverlay`

**Step 4: Run tests to verify they pass**

```bash
cargo test -p nes-desktop selected_slot_hotkeys_target_current_slot --bin nes-desktop
cargo test -p nes-desktop overlay_escape_toggles_pause_menu_instead_of_exit --bin nes-desktop
```

Expected: PASS

**Step 5: Commit**

```bash
git add crates/nes-desktop/src/main.rs
git commit -m "refactor: extract rom session helpers and overlay action wiring"
```

---

## Task 6: Integrate Native Menu, Modal Overlay, And Slot Actions Into The Event Loop

**Files:**
- Modify: `crates/nes-desktop/src/main.rs`
- Modify: `crates/nes-desktop/src/manual_state.rs` if small helper gaps remain

**Step 1: Write the failing tests**

In `crates/nes-desktop/src/main.rs` tests, add:

```rust
#[test]
fn overlay_blocks_gameplay_button_commands_while_open() {
    let mut keyboard_bits = 0_u8;
    let decision = apply_overlay_keyboard_input(
        true,
        VirtualKeyCode::Z,
        true,
        &mut keyboard_bits,
    );

    assert_eq!(decision, Some(AppAction::Resume));
    assert_eq!(keyboard_bits, 0);
}

#[test]
fn rollback_disables_stateful_menu_actions() {
    let err = validate_action_allowed(AppAction::OpenRom, true).unwrap_err();
    assert!(err.contains("unavailable while netplay/rollback is active"));
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -p nes-desktop overlay_blocks_gameplay_button_commands_while_open --bin nes-desktop
cargo test -p nes-desktop rollback_disables_stateful_menu_actions --bin nes-desktop
```

Expected: FAIL because overlay-aware input filtering and action gating do not exist.

**Step 3: Write minimal implementation**

- In `crates/nes-desktop/src/main.rs`:
  - create the native menu after window creation
  - install the menu on the window via `muda`
  - instantiate `OverlayModel::new(5)`
  - on `ToggleOverlay`:
    - open/close overlay
    - pause/resume core
    - clear audio queue when entering overlay
  - poll native menu events during `MainEventsCleared` and convert them to `AppAction`
  - when overlay is open:
    - suppress gameplay button handling
    - route arrow keys / enter / escape to overlay navigation
    - skip frame advancement
  - implement `execute_app_action(...)` handling:
    - `Resume`
    - `OpenRom`
    - `SaveSlot(slot)`
    - `LoadSlot(slot)`
    - `Reset`
    - `Quit`
  - `OpenRom` path:
    - call `pick_rom_path()`
    - load the selected ROM
    - refresh `LoadedRomSession`
    - update window title to include ROM name and pause state
  - `SaveSlot` / `LoadSlot` path:
    - use numbered slot helpers
    - update overlay status message on success/failure
  - render path:
    - copy `frame_rgba` to `pixels.frame_mut()`
    - if overlay is open, draw the overlay over the copied frame
    - render

**Step 4: Run focused tests to verify green**

```bash
cargo test -p nes-desktop overlay_blocks_gameplay_button_commands_while_open --bin nes-desktop
cargo test -p nes-desktop rollback_disables_stateful_menu_actions --bin nes-desktop
cargo test -p nes-desktop classify_keyboard_input_covers_exit_rewind_rollback_and_core_paths --bin nes-desktop
```

Expected: PASS after updating the old keyboard test expectations for `Escape`.

**Step 5: Commit**

```bash
git add crates/nes-desktop/src/main.rs crates/nes-desktop/src/manual_state.rs
git commit -m "feat: integrate native menu and modal overlay into desktop runtime"
```

---

## Task 7: Final Verification And Help Text Cleanup

**Files:**
- Modify: `crates/nes-desktop/src/main.rs`
- Modify: `crates/nes-desktop/tests/cli_help.rs` if usage/help text changes

**Step 1: Update startup/help text**

- Replace the old controls row with something like:

```text
keyboard Z=A, X=B, Enter=Start, RightShift=Select, Arrows=D-pad, Esc=Menu, F5=Save Slot, F8=Load Slot
```

- Add visible overlay hint text if needed.

**Step 2: Run the full verification set**

```bash
cargo fmt --all
cargo test -p nes-desktop --lib
cargo test -p nes-desktop --bin nes-desktop
cargo test -p nes-desktop --test cli_help
cargo test -p nes-core
rg -n "TODO|FIXME|Stub:" crates/nes-desktop crates/nes-core -S
```

Expected:
- formatting succeeds
- all targeted tests pass
- no TODO/FIXME/Stub leftovers in affected areas

**Step 3: Review the diff**

```bash
git diff --stat
git diff -- crates/nes-desktop/Cargo.toml crates/nes-desktop/src
```

Expected: one coherent feature set with no accidental churn outside `nes-desktop`.

**Step 4: Commit**

```bash
git add crates/nes-desktop/Cargo.toml crates/nes-desktop/src crates/nes-desktop/tests/cli_help.rs
git commit -m "feat: add hybrid desktop menu with rom picker and save slots"
```

---

## Notes For Implementation

- Do not add a heavyweight UI framework. `pixels + bitmap text` is enough for `v1`.
- Do not let the overlay mutate controller state while open.
- Keep menu construction and action parsing in separate pure helpers so tests stay cheap.
- Preserve existing RTA and rollback restrictions. State-changing actions should fail loudly and clearly when disallowed.
- Treat ROM switching as a full session reload: core, hashes, slot metadata, audio queue, rewind state, and title should all refresh together.
