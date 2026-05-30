//! Modal pause overlay model and bitmap renderer for `nes-desktop`.

use font8x8::{BASIC_FONTS, UnicodeFonts};
use std::borrow::Cow;
use winit::event::VirtualKeyCode;

use crate::actions::AppAction;

const GLYPH_SIZE: usize = 8;
const GLYPH_SPACING: usize = 1;
const LINE_HEIGHT: usize = 10;
const PANEL_MARGIN: usize = 16;
const PANEL_PADDING: usize = 12;
const CHEAT_CODE_MAX_LEN: usize = 8;

const COLOR_BACKDROP: [u8; 4] = [0, 0, 0, 255];
const COLOR_PANEL: [u8; 4] = [28, 30, 36, 255];
const COLOR_PANEL_BORDER: [u8; 4] = [94, 104, 124, 255];
const COLOR_TEXT: [u8; 4] = [235, 239, 247, 255];
const COLOR_SUBTEXT: [u8; 4] = [170, 178, 194, 255];
const COLOR_HIGHLIGHT: [u8; 4] = [74, 122, 197, 255];
const COLOR_STATUS: [u8; 4] = [207, 173, 91, 255];

/// Which overlay screen is currently visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayPanel {
    /// The primary top-level menu view.
    MainMenu,
    /// The view for managing cheat codes.
    Cheats,
}

/// Selectable entries in the main pause menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainMenuSelection {
    /// Requests the emulation to resume running.
    Resume,
    /// Requests the application to open a new ROM file.
    OpenRom,
    /// Requests navigation to the cheats management view.
    OpenCheats,
    /// Requests saving state to the specified slot number.
    SaveSlot(u8),
    /// Requests loading state from the specified slot number.
    LoadSlot(u8),
    /// Requests a hard reset of the emulator.
    Reset,
    /// Requests the application to exit.
    Quit,
}

/// Selectable entries in the cheats panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheatsSelection {
    /// Interaction representing the user's intent to add a new cheat code.
    AddCode,
    /// Interaction with a specific existing cheat code.
    Cheat(usize),
}

/// Unified selection state for the active overlay panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlaySelection {
    /// An action generated from the main menu view.
    Main(MainMenuSelection),
    /// An action generated from the cheats view.
    Cheats(CheatsSelection),
}

/// Commands emitted from the overlay state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayCommand {
    /// A high-level application action to be handled by the host.
    AppAction(AppAction),
    /// Requests toggling the active state of a specific cheat code.
    ToggleCheat(usize),
    /// Requests the permanent removal of a specific cheat code.
    RemoveCheat(usize),
    /// Requests the addition of a new cheat code string.
    SubmitCheatCode(String),
}

/// Lightweight slot summary used by the overlay renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlaySlotSummary {
    /// The numerical identifier of the slot.
    pub slot: u8,
    /// A short, human-readable label indicating the slot's status (e.g., 'Empty').
    pub status_label: &'static str,
    /// The last modified time of the slot, if available.
    pub modified_unix_secs: Option<u64>,
}

impl OverlaySlotSummary {
    /// Returns a human-readable suffix for rendering beside slot actions.
    pub fn render_suffix(&self, out: &mut String) {
        use std::fmt::Write;
        if let Some(secs) = self.modified_unix_secs {
            let _ = write!(out, "{} ({secs})", self.status_label);
        } else {
            out.push_str(self.status_label);
        }
    }
}

/// Lightweight cheat summary used by the overlay renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayCheatSummary<'a> {
    /// The raw text of the cheat code (e.g., Game Genie format).
    pub raw_code: &'a str,
    /// Whether the cheat is currently active and applied to the emulator.
    pub enabled: bool,
}

/// Modal pause overlay state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayModel {
    open: bool,
    panel: OverlayPanel,
    selected_slot: u8,
    main_selection_index: usize,
    cheats_selection_index: usize,
    add_cheat_input: Option<String>,
    main_entries: Vec<MainMenuSelection>,
    status_message: Option<Cow<'static, str>>,
}

impl OverlayModel {
    /// Creates a new overlay model with entries for the given slot count.
    #[must_use]
    pub fn new(slot_count: u8) -> Self {
        let capped_slot_count = slot_count.max(1);
        let mut main_entries = Vec::with_capacity(usize::from(capped_slot_count) * 2 + 5);
        main_entries.push(MainMenuSelection::Resume);
        main_entries.push(MainMenuSelection::OpenRom);
        main_entries.push(MainMenuSelection::OpenCheats);
        for slot in 1..=capped_slot_count {
            main_entries.push(MainMenuSelection::SaveSlot(slot));
        }
        for slot in 1..=capped_slot_count {
            main_entries.push(MainMenuSelection::LoadSlot(slot));
        }
        main_entries.push(MainMenuSelection::Reset);
        main_entries.push(MainMenuSelection::Quit);

        Self {
            open: false,
            panel: OverlayPanel::MainMenu,
            selected_slot: 1,
            main_selection_index: 0,
            cheats_selection_index: 0,
            add_cheat_input: None,
            main_entries,
            status_message: None,
        }
    }

    /// Returns whether the overlay is currently open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.open
    }

    /// Returns the currently visible overlay panel.
    #[must_use]
    pub const fn panel(&self) -> OverlayPanel {
        self.panel
    }

    /// Opens the overlay.
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Closes the overlay and resets transient UI state.
    pub fn close(&mut self) {
        self.open = false;
        self.panel = OverlayPanel::MainMenu;
        self.add_cheat_input = None;
    }

    /// Opens the cheats panel and keeps the overlay visible.
    pub fn open_cheats_panel(&mut self) {
        self.open = true;
        self.panel = OverlayPanel::Cheats;
        self.add_cheat_input = None;
    }

    /// Returns whether the add-cheat modal is active.
    #[must_use]
    pub fn is_add_cheat_modal_open(&self) -> bool {
        self.add_cheat_input.is_some()
    }

    /// Closes the add-cheat modal without changing the active panel.
    pub fn close_add_cheat_modal(&mut self) {
        self.add_cheat_input = None;
    }

    /// Returns the current input buffer when the add-cheat modal is active.
    #[must_use]
    pub fn cheat_input_buffer(&self) -> Option<&str> {
        self.add_cheat_input.as_deref()
    }

    /// Returns the currently selected entry for the active panel.
    #[must_use]
    pub fn selection(&self, cheat_count: usize) -> OverlaySelection {
        match self.panel {
            OverlayPanel::MainMenu => {
                OverlaySelection::Main(self.main_entries[self.main_selection_index])
            }
            OverlayPanel::Cheats => OverlaySelection::Cheats(self.cheats_selection(cheat_count)),
        }
    }

    /// Moves selection to the previous entry, wrapping at the top.
    pub fn move_prev(&mut self, cheat_count: usize) {
        match self.panel {
            OverlayPanel::MainMenu => {
                if self.main_selection_index == 0 {
                    self.main_selection_index = self.main_entries.len().saturating_sub(1);
                } else {
                    self.main_selection_index -= 1;
                }
                self.sync_selected_slot_with_main_selection();
            }
            OverlayPanel::Cheats => {
                let entry_count = cheat_count.saturating_add(1);
                if self.cheats_selection_index == 0 {
                    self.cheats_selection_index = entry_count.saturating_sub(1);
                } else {
                    self.cheats_selection_index -= 1;
                }
            }
        }
    }

    /// Moves selection to the next entry, wrapping at the bottom.
    pub fn move_next(&mut self, cheat_count: usize) {
        match self.panel {
            OverlayPanel::MainMenu => {
                self.main_selection_index =
                    (self.main_selection_index + 1) % self.main_entries.len();
                self.sync_selected_slot_with_main_selection();
            }
            OverlayPanel::Cheats => {
                let entry_count = cheat_count.saturating_add(1);
                self.cheats_selection_index =
                    (self.cheats_selection_index + 1) % entry_count.max(1);
            }
        }
    }

    /// Handles one overlay key press and emits a high-level command when needed.
    #[must_use]
    pub fn handle_key(
        &mut self,
        key: VirtualKeyCode,
        pressed: bool,
        cheat_count: usize,
    ) -> Option<OverlayCommand> {
        if !self.open || !pressed {
            return None;
        }
        if self.is_add_cheat_modal_open() {
            return self.handle_add_cheat_modal_key(key);
        }

        match self.panel {
            OverlayPanel::MainMenu => self.handle_main_menu_key(key),
            OverlayPanel::Cheats => self.handle_cheats_panel_key(key, cheat_count),
        }
    }

    /// Returns the slot most recently chosen through main menu slot actions.
    #[must_use]
    pub const fn selected_slot(&self) -> u8 {
        self.selected_slot
    }

    /// Repositions the main-menu selection to the first entry for the given slot.
    pub fn focus_slot(&mut self, slot: u8, save_action: bool) {
        if let Some(index) = self
            .main_entries
            .iter()
            .position(|entry| match (save_action, entry) {
                (true, MainMenuSelection::SaveSlot(candidate)) => *candidate == slot,
                (false, MainMenuSelection::LoadSlot(candidate)) => *candidate == slot,
                _ => false,
            })
        {
            self.main_selection_index = index;
            self.sync_selected_slot_with_main_selection();
        }
    }

    /// Selects the given cheat row in the cheats panel.
    pub fn focus_cheat(&mut self, cheat_index: usize) {
        self.cheats_selection_index = cheat_index.saturating_add(1);
    }

    /// Sets a user-facing status message shown at the bottom of the overlay.
    pub fn set_status_message(&mut self, message: impl Into<Cow<'static, str>>) {
        self.status_message = Some(message.into());
    }

    /// Clears any existing status message.
    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }

    /// Returns the current status message, if any.
    #[must_use]
    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    fn handle_add_cheat_modal_key(&mut self, key: VirtualKeyCode) -> Option<OverlayCommand> {
        match key {
            VirtualKeyCode::Escape => {
                self.add_cheat_input = None;
                None
            }
            VirtualKeyCode::Back => {
                if let Some(buffer) = self.add_cheat_input.as_mut() {
                    buffer.pop();
                }
                None
            }
            VirtualKeyCode::Return => {
                let buffer = self.add_cheat_input.as_ref()?;
                if buffer.is_empty() {
                    return None;
                }
                Some(OverlayCommand::SubmitCheatCode(buffer.clone()))
            }
            _ => {
                if let Some(ch) = key_to_cheat_char(key)
                    && let Some(buffer) = self.add_cheat_input.as_mut()
                    && buffer.len() < CHEAT_CODE_MAX_LEN
                {
                    buffer.push(ch);
                }
                None
            }
        }
    }

    fn handle_main_menu_key(&mut self, key: VirtualKeyCode) -> Option<OverlayCommand> {
        match key {
            VirtualKeyCode::Escape => Some(OverlayCommand::AppAction(AppAction::Resume)),
            VirtualKeyCode::Up => {
                self.move_prev(0);
                None
            }
            VirtualKeyCode::Down => {
                self.move_next(0);
                None
            }
            VirtualKeyCode::Return => Some(OverlayCommand::AppAction(self.main_action())),
            VirtualKeyCode::F5 => Some(OverlayCommand::AppAction(AppAction::SaveSlot(
                self.selected_slot,
            ))),
            VirtualKeyCode::F8 => Some(OverlayCommand::AppAction(AppAction::LoadSlot(
                self.selected_slot,
            ))),
            _ => None,
        }
    }

    fn handle_cheats_panel_key(
        &mut self,
        key: VirtualKeyCode,
        cheat_count: usize,
    ) -> Option<OverlayCommand> {
        match key {
            VirtualKeyCode::Escape => {
                self.panel = OverlayPanel::MainMenu;
                None
            }
            VirtualKeyCode::Up => {
                self.move_prev(cheat_count);
                None
            }
            VirtualKeyCode::Down => {
                self.move_next(cheat_count);
                None
            }
            VirtualKeyCode::Return => {
                if matches!(self.cheats_selection(cheat_count), CheatsSelection::AddCode) {
                    self.add_cheat_input = Some(String::new());
                }
                None
            }
            VirtualKeyCode::Space => match self.cheats_selection(cheat_count) {
                CheatsSelection::AddCode => None,
                CheatsSelection::Cheat(index) => Some(OverlayCommand::ToggleCheat(index)),
            },
            VirtualKeyCode::Delete => match self.cheats_selection(cheat_count) {
                CheatsSelection::AddCode => None,
                CheatsSelection::Cheat(index) => Some(OverlayCommand::RemoveCheat(index)),
            },
            _ => None,
        }
    }

    fn main_action(&self) -> AppAction {
        match self.main_entries[self.main_selection_index] {
            MainMenuSelection::Resume => AppAction::Resume,
            MainMenuSelection::OpenRom => AppAction::OpenRom,
            MainMenuSelection::OpenCheats => AppAction::OpenCheats,
            MainMenuSelection::SaveSlot(slot) => AppAction::SaveSlot(slot),
            MainMenuSelection::LoadSlot(slot) => AppAction::LoadSlot(slot),
            MainMenuSelection::Reset => AppAction::Reset,
            MainMenuSelection::Quit => AppAction::Quit,
        }
    }

    fn cheats_selection(&self, cheat_count: usize) -> CheatsSelection {
        let max_index = cheat_count;
        let bounded_index = self.cheats_selection_index.min(max_index);
        if bounded_index == 0 {
            CheatsSelection::AddCode
        } else {
            CheatsSelection::Cheat(bounded_index - 1)
        }
    }

    fn sync_selected_slot_with_main_selection(&mut self) {
        match self.main_entries[self.main_selection_index] {
            MainMenuSelection::SaveSlot(slot) | MainMenuSelection::LoadSlot(slot) => {
                self.selected_slot = slot;
            }
            MainMenuSelection::Resume
            | MainMenuSelection::OpenRom
            | MainMenuSelection::OpenCheats
            | MainMenuSelection::Reset
            | MainMenuSelection::Quit => {}
        }
    }
}

/// Draws a filled rectangle into an RGBA buffer.
pub fn fill_rect(
    frame: &mut [u8],
    frame_width: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: [u8; 4],
) {
    if frame_width == 0 || width == 0 || height == 0 {
        return;
    }
    let frame_height = frame.len() / 4 / frame_width;
    let x_end = x.saturating_add(width).min(frame_width);
    let y_end = y.saturating_add(height).min(frame_height);
    for row in y..y_end {
        for col in x..x_end {
            set_pixel(frame, frame_width, col, row, color);
        }
    }
}

/// Draws bitmap text into an RGBA buffer.
pub fn draw_text(
    frame: &mut [u8],
    frame_width: usize,
    x: usize,
    y: usize,
    text: &str,
    color: [u8; 4],
) {
    let mut cursor_x = x;
    let mut cursor_y = y;
    for ch in text.chars() {
        if ch == '\n' {
            cursor_x = x;
            cursor_y = cursor_y.saturating_add(LINE_HEIGHT);
            continue;
        }
        draw_char(frame, frame_width, cursor_x, cursor_y, ch, color);
        cursor_x = cursor_x.saturating_add(GLYPH_SIZE + GLYPH_SPACING);
    }
}

/// Draws the full modal overlay on top of the existing frame.
///
/// **Bolt Optimization:** The `slot_summaries` and `cheat_summaries` arguments use
/// lazy iterators instead of collecting into `Vec` slices. This eliminates repeated heap
/// allocations and mappings for the UI list states on every redraw tick.
pub fn draw_overlay<'a>(
    frame: &mut [u8],
    frame_width: usize,
    frame_height: usize,
    model: &OverlayModel,
    slot_summaries: impl Iterator<Item = OverlaySlotSummary> + Clone,
    mut cheat_summaries: impl Iterator<Item = OverlayCheatSummary<'a>>,
    cheat_count: usize,
) {
    if !model.is_open() || frame_width == 0 || frame_height == 0 {
        return;
    }

    dim_frame(frame);

    let panel_x = PANEL_MARGIN.min(frame_width);
    let panel_y = PANEL_MARGIN.min(frame_height);
    let panel_width = frame_width.saturating_sub(panel_x * 2);
    let panel_height = frame_height.saturating_sub(panel_y * 2);
    fill_rect(
        frame,
        frame_width,
        panel_x,
        panel_y,
        panel_width,
        panel_height,
        COLOR_PANEL,
    );
    draw_frame_border(
        frame,
        frame_width,
        panel_x,
        panel_y,
        panel_width,
        panel_height,
    );

    let text_x = panel_x.saturating_add(PANEL_PADDING);
    let mut text_y = panel_y.saturating_add(PANEL_PADDING);

    let mut text_buffer = String::with_capacity(64);

    match model.panel() {
        OverlayPanel::MainMenu => {
            draw_text(frame, frame_width, text_x, text_y, "PAUSED", COLOR_TEXT);
            text_y = text_y.saturating_add(LINE_HEIGHT + 2);
            draw_text(
                frame,
                frame_width,
                text_x,
                text_y,
                "Resume | Open ROM | Cheats | Save/Load Slots | Reset | Quit",
                COLOR_SUBTEXT,
            );
            text_y = text_y.saturating_add(LINE_HEIGHT + 6);

            for (index, entry) in model.main_entries.iter().enumerate() {
                let is_selected = index == model.main_selection_index;
                let line_top = text_y.saturating_sub(2);
                if is_selected {
                    fill_rect(
                        frame,
                        frame_width,
                        text_x.saturating_sub(4),
                        line_top,
                        panel_width
                            .saturating_sub(PANEL_PADDING * 2)
                            .min(frame_width),
                        LINE_HEIGHT,
                        COLOR_HIGHLIGHT,
                    );
                }

                text_buffer.clear();
                main_entry_label(*entry, slot_summaries.clone(), &mut text_buffer);
                draw_text(
                    frame,
                    frame_width,
                    text_x,
                    text_y,
                    &text_buffer,
                    if is_selected {
                        COLOR_TEXT
                    } else {
                        COLOR_SUBTEXT
                    },
                );
                text_y = text_y.saturating_add(LINE_HEIGHT);
            }
        }
        OverlayPanel::Cheats => {
            draw_text(frame, frame_width, text_x, text_y, "CHEATS", COLOR_TEXT);
            text_y = text_y.saturating_add(LINE_HEIGHT + 2);
            draw_text(
                frame,
                frame_width,
                text_x,
                text_y,
                "Enter=Add | Space=Toggle | Del=Remove | Esc=Back",
                COLOR_SUBTEXT,
            );
            text_y = text_y.saturating_add(LINE_HEIGHT + 6);

            for index in 0..cheat_count.saturating_add(1) {
                let is_selected = index == model.cheats_selection_index.min(cheat_count);
                let line_top = text_y.saturating_sub(2);
                if is_selected {
                    fill_rect(
                        frame,
                        frame_width,
                        text_x.saturating_sub(4),
                        line_top,
                        panel_width
                            .saturating_sub(PANEL_PADDING * 2)
                            .min(frame_width),
                        LINE_HEIGHT,
                        COLOR_HIGHLIGHT,
                    );
                }

                text_buffer.clear();
                if index == 0 {
                    text_buffer.push_str("Add Code");
                } else if let Some(summary) = cheat_summaries.next() {
                    cheat_label(&summary, &mut text_buffer);
                }
                draw_text(
                    frame,
                    frame_width,
                    text_x,
                    text_y,
                    &text_buffer,
                    if is_selected {
                        COLOR_TEXT
                    } else {
                        COLOR_SUBTEXT
                    },
                );
                text_y = text_y.saturating_add(LINE_HEIGHT);
            }
        }
    }

    if let Some(message) = model.status_message() {
        let footer_y = panel_y
            .saturating_add(panel_height)
            .saturating_sub(PANEL_PADDING + LINE_HEIGHT);
        draw_text(frame, frame_width, text_x, footer_y, message, COLOR_STATUS);
    }

    if let Some(buffer) = model.cheat_input_buffer() {
        draw_add_cheat_modal(frame, frame_width, frame_height, buffer);
    }
}

fn draw_add_cheat_modal(frame: &mut [u8], frame_width: usize, frame_height: usize, buffer: &str) {
    let modal_width = 200.min(frame_width.saturating_sub(PANEL_MARGIN));
    let modal_height = 54.min(frame_height.saturating_sub(PANEL_MARGIN));
    let modal_x = (frame_width.saturating_sub(modal_width)) / 2;
    let modal_y = (frame_height.saturating_sub(modal_height)) / 2;
    fill_rect(
        frame,
        frame_width,
        modal_x,
        modal_y,
        modal_width,
        modal_height,
        COLOR_PANEL,
    );
    draw_frame_border(
        frame,
        frame_width,
        modal_x,
        modal_y,
        modal_width,
        modal_height,
    );
    let text_x = modal_x.saturating_add(PANEL_PADDING);
    let text_y = modal_y.saturating_add(PANEL_PADDING);
    draw_text(
        frame,
        frame_width,
        text_x,
        text_y,
        "Enter cheat code:",
        COLOR_TEXT,
    );

    // ⚡ Bolt Optimization:
    // Eliminates a per-frame `String` heap allocation (from `format!("{}_", buffer)`)
    // by drawing the input text and the cursor separately.
    let input_y = text_y.saturating_add(LINE_HEIGHT + 4);
    draw_text(frame, frame_width, text_x, input_y, buffer, COLOR_STATUS);
    let cursor_x = text_x.saturating_add(buffer.chars().count() * (GLYPH_SIZE + GLYPH_SPACING));
    draw_text(frame, frame_width, cursor_x, input_y, "_", COLOR_STATUS);
}

fn draw_frame_border(
    frame: &mut [u8],
    frame_width: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) {
    fill_rect(frame, frame_width, x, y, width, 1, COLOR_PANEL_BORDER);
    fill_rect(
        frame,
        frame_width,
        x,
        y.saturating_add(height.saturating_sub(1)),
        width,
        1,
        COLOR_PANEL_BORDER,
    );
    fill_rect(frame, frame_width, x, y, 1, height, COLOR_PANEL_BORDER);
    fill_rect(
        frame,
        frame_width,
        x.saturating_add(width.saturating_sub(1)),
        y,
        1,
        height,
        COLOR_PANEL_BORDER,
    );
}

fn dim_frame(frame: &mut [u8]) {
    for pixel in frame.chunks_exact_mut(4) {
        pixel[0] = pixel[0] / 3 + COLOR_BACKDROP[0] / 4;
        pixel[1] = pixel[1] / 3 + COLOR_BACKDROP[1] / 4;
        pixel[2] = pixel[2] / 3 + COLOR_BACKDROP[2] / 4;
        pixel[3] = 255;
    }
}

fn draw_char(frame: &mut [u8], frame_width: usize, x: usize, y: usize, ch: char, color: [u8; 4]) {
    let glyph = BASIC_FONTS
        .get(ch)
        .or_else(|| BASIC_FONTS.get('?'))
        .unwrap_or([0; GLYPH_SIZE]);
    for (row_idx, row_bits) in glyph.iter().copied().enumerate() {
        for col_idx in 0..GLYPH_SIZE {
            if (row_bits >> col_idx) & 1 == 1 {
                set_pixel(
                    frame,
                    frame_width,
                    x.saturating_add(col_idx),
                    y.saturating_add(row_idx),
                    color,
                );
            }
        }
    }
}

fn set_pixel(frame: &mut [u8], frame_width: usize, x: usize, y: usize, color: [u8; 4]) {
    if frame_width == 0 {
        return;
    }
    let frame_height = frame.len() / 4 / frame_width;
    if x >= frame_width || y >= frame_height {
        return;
    }
    let offset = (y * frame_width + x) * 4;
    if let Some(pixel) = frame.get_mut(offset..offset + 4) {
        pixel.copy_from_slice(&color);
    }
}

/// ⚡ Bolt Optimization:
/// Eliminates intermediate `String` heap allocations per frame while the overlay is open
/// by writing directly to a reusable `String` buffer via `std::fmt::Write`.
fn main_entry_label(
    entry: MainMenuSelection,
    mut slot_summaries: impl Iterator<Item = OverlaySlotSummary>,
    out: &mut String,
) {
    use std::fmt::Write;
    match entry {
        MainMenuSelection::Resume => out.push_str("Resume"),
        MainMenuSelection::OpenRom => out.push_str("Open ROM..."),
        MainMenuSelection::OpenCheats => out.push_str("Cheats..."),
        MainMenuSelection::SaveSlot(slot) => {
            let _ = write!(out, "Save Slot {slot}: ");
            slot_suffix(slot, &mut slot_summaries, out);
        }
        MainMenuSelection::LoadSlot(slot) => {
            let _ = write!(out, "Load Slot {slot}: ");
            slot_suffix(slot, &mut slot_summaries, out);
        }
        MainMenuSelection::Reset => out.push_str("Reset"),
        MainMenuSelection::Quit => out.push_str("Quit"),
    }
}

fn cheat_label(summary: &OverlayCheatSummary, out: &mut String) {
    use std::fmt::Write;
    let _ = write!(
        out,
        "{} {}",
        if summary.enabled { "ON " } else { "OFF" },
        summary.raw_code
    );
}

fn slot_suffix(
    slot: u8,
    slot_summaries: &mut impl Iterator<Item = OverlaySlotSummary>,
    out: &mut String,
) {
    if let Some(summary) = slot_summaries.find(|summary| summary.slot == slot) {
        summary.render_suffix(out);
    } else {
        out.push_str("Unknown");
    }
}

fn key_to_cheat_char(key: VirtualKeyCode) -> Option<char> {
    match key {
        VirtualKeyCode::A => Some('A'),
        VirtualKeyCode::E => Some('E'),
        VirtualKeyCode::G => Some('G'),
        VirtualKeyCode::I => Some('I'),
        VirtualKeyCode::K => Some('K'),
        VirtualKeyCode::L => Some('L'),
        VirtualKeyCode::N => Some('N'),
        VirtualKeyCode::O => Some('O'),
        VirtualKeyCode::P => Some('P'),
        VirtualKeyCode::S => Some('S'),
        VirtualKeyCode::T => Some('T'),
        VirtualKeyCode::U => Some('U'),
        VirtualKeyCode::V => Some('V'),
        VirtualKeyCode::X => Some('X'),
        VirtualKeyCode::Y => Some('Y'),
        VirtualKeyCode::Z => Some('Z'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CheatsSelection, MainMenuSelection, OverlayCheatSummary, OverlayCommand, OverlayModel,
        OverlayPanel, OverlaySelection, OverlaySlotSummary, draw_overlay, draw_text,
    };
    use crate::actions::AppAction;
    use winit::event::VirtualKeyCode;

    #[test]
    fn main_menu_navigation_wraps_and_tracks_selected_slot() {
        let mut overlay = OverlayModel::new(3);
        assert_eq!(
            overlay.selection(0),
            OverlaySelection::Main(MainMenuSelection::Resume)
        );

        overlay.move_prev(0);
        assert_eq!(
            overlay.selection(0),
            OverlaySelection::Main(MainMenuSelection::Quit)
        );

        overlay.move_next(0);
        assert_eq!(
            overlay.selection(0),
            OverlaySelection::Main(MainMenuSelection::Resume)
        );

        overlay.move_next(0);
        overlay.move_next(0);
        overlay.move_next(0);
        assert_eq!(
            overlay.selection(0),
            OverlaySelection::Main(MainMenuSelection::SaveSlot(1))
        );

        assert_eq!(overlay.selected_slot(), 1);
    }

    #[test]
    fn moving_main_menu_slot_selection_updates_selected_slot() {
        let mut overlay = OverlayModel::new(3);

        overlay.move_next(0);
        overlay.move_next(0);
        overlay.move_next(0);
        overlay.move_next(0);
        assert_eq!(
            overlay.selection(0),
            OverlaySelection::Main(MainMenuSelection::SaveSlot(2))
        );
        assert_eq!(overlay.selected_slot(), 2);

        overlay.move_next(0);
        overlay.move_next(0);
        assert_eq!(
            overlay.selection(0),
            OverlaySelection::Main(MainMenuSelection::LoadSlot(1))
        );
        assert_eq!(overlay.selected_slot(), 1);
    }

    #[test]
    fn draw_text_marks_pixels_inside_target_buffer() {
        let mut frame = vec![0_u8; 64 * 64 * 4];
        draw_text(&mut frame, 64, 2, 2, "NES", [255, 255, 255, 255]);
        assert!(frame.iter().any(|component| *component != 0));
    }

    #[test]
    fn draw_overlay_renders_main_panel_selection_and_status_message() {
        let mut frame = vec![16_u8; 128 * 96 * 4];
        let mut overlay = OverlayModel::new(2);
        overlay.open();
        overlay.move_next(0);
        overlay.set_status_message("Slot 1 loaded");
        let slots = vec![
            OverlaySlotSummary {
                slot: 1,
                status_label: "Saved",
                modified_unix_secs: Some(1_700_000_000),
            },
            OverlaySlotSummary {
                slot: 2,
                status_label: "Empty",
                modified_unix_secs: None,
            },
        ];

        draw_overlay(
            &mut frame,
            128,
            96,
            &overlay,
            slots.into_iter(),
            std::iter::empty(),
            0,
        );

        assert!(
            frame.iter().any(|component| *component != 16),
            "overlay draw should mutate the frame"
        );
    }

    #[test]
    fn cheats_panel_navigation_wraps_and_escape_returns_to_main_menu() {
        let mut overlay = OverlayModel::new(2);
        overlay.open_cheats_panel();

        assert_eq!(overlay.panel(), OverlayPanel::Cheats);
        assert_eq!(
            overlay.selection(2),
            OverlaySelection::Cheats(CheatsSelection::AddCode)
        );

        overlay.move_prev(2);
        assert_eq!(
            overlay.selection(2),
            OverlaySelection::Cheats(CheatsSelection::Cheat(1))
        );

        let action = overlay.handle_key(VirtualKeyCode::Escape, true, 2);
        assert_eq!(action, None);
        assert_eq!(overlay.panel(), OverlayPanel::MainMenu);
        assert_eq!(
            overlay.selection(2),
            OverlaySelection::Main(MainMenuSelection::Resume)
        );
    }

    #[test]
    fn add_cheat_modal_collects_input_and_submits_uppercase_code() {
        let mut overlay = OverlayModel::new(1);
        overlay.open_cheats_panel();

        let action = overlay.handle_key(VirtualKeyCode::Return, true, 0);
        assert_eq!(action, None);
        assert!(overlay.is_add_cheat_modal_open());

        assert_eq!(overlay.handle_key(VirtualKeyCode::G, true, 0), None);
        assert_eq!(overlay.handle_key(VirtualKeyCode::O, true, 0), None);
        assert_eq!(overlay.handle_key(VirtualKeyCode::S, true, 0), None);
        assert_eq!(overlay.handle_key(VirtualKeyCode::S, true, 0), None);
        assert_eq!(overlay.handle_key(VirtualKeyCode::I, true, 0), None);
        assert_eq!(overlay.handle_key(VirtualKeyCode::P, true, 0), None);
        assert_eq!(overlay.cheat_input_buffer(), Some("GOSSIP"));

        let action = overlay.handle_key(VirtualKeyCode::Return, true, 0);
        assert_eq!(
            action,
            Some(OverlayCommand::SubmitCheatCode("GOSSIP".to_owned()))
        );
        assert!(overlay.is_add_cheat_modal_open());
        overlay.close_add_cheat_modal();
        assert!(!overlay.is_add_cheat_modal_open());
    }

    #[test]
    fn cheats_panel_supports_toggle_and_remove_shortcuts() {
        let mut overlay = OverlayModel::new(1);
        overlay.open_cheats_panel();
        overlay.move_next(2);

        assert_eq!(
            overlay.selection(2),
            OverlaySelection::Cheats(CheatsSelection::Cheat(0))
        );
        assert_eq!(
            overlay.handle_key(VirtualKeyCode::Space, true, 2),
            Some(OverlayCommand::ToggleCheat(0))
        );
        assert_eq!(
            overlay.handle_key(VirtualKeyCode::Delete, true, 2),
            Some(OverlayCommand::RemoveCheat(0))
        );
    }

    #[test]
    fn draw_overlay_renders_cheats_panel_and_input_modal() {
        let mut frame = vec![32_u8; 160 * 120 * 4];
        let mut overlay = OverlayModel::new(1);
        overlay.open_cheats_panel();
        let _ = overlay.handle_key(VirtualKeyCode::Return, true, 0);
        let _ = overlay.handle_key(VirtualKeyCode::G, true, 0);
        let slots = vec![OverlaySlotSummary {
            slot: 1,
            status_label: "Empty",
            modified_unix_secs: None,
        }];
        let cheats = vec![OverlayCheatSummary {
            raw_code: "GOSSIP",
            enabled: true,
        }];

        draw_overlay(
            &mut frame,
            160,
            120,
            &overlay,
            slots.into_iter(),
            cheats.into_iter(),
            1,
        );

        assert!(
            frame.iter().any(|component| *component != 32),
            "cheats overlay draw should mutate the frame"
        );
    }

    #[test]
    fn main_menu_can_activate_cheats_action() {
        let mut overlay = OverlayModel::new(1);
        overlay.open();
        overlay.move_next(0);
        overlay.move_next(0);

        assert_eq!(
            overlay.selection(0),
            OverlaySelection::Main(MainMenuSelection::OpenCheats)
        );
        assert_eq!(
            overlay.handle_key(VirtualKeyCode::Return, true, 0),
            Some(OverlayCommand::AppAction(AppAction::OpenCheats))
        );
    }
}
