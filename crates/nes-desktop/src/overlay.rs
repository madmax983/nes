//! Modal pause overlay model and bitmap renderer for `nes-desktop`.

use font8x8::{BASIC_FONTS, UnicodeFonts};

const GLYPH_SIZE: usize = 8;
const GLYPH_SPACING: usize = 1;
const LINE_HEIGHT: usize = 10;
const PANEL_MARGIN: usize = 16;
const PANEL_PADDING: usize = 12;

const COLOR_BACKDROP: [u8; 4] = [0, 0, 0, 255];
const COLOR_PANEL: [u8; 4] = [28, 30, 36, 255];
const COLOR_PANEL_BORDER: [u8; 4] = [94, 104, 124, 255];
const COLOR_TEXT: [u8; 4] = [235, 239, 247, 255];
const COLOR_SUBTEXT: [u8; 4] = [170, 178, 194, 255];
const COLOR_HIGHLIGHT: [u8; 4] = [74, 122, 197, 255];
const COLOR_STATUS: [u8; 4] = [207, 173, 91, 255];

/// A selectable overlay entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlaySelection {
    Resume,
    OpenRom,
    SaveSlot(u8),
    LoadSlot(u8),
    Reset,
    Quit,
}

/// Alias for overlay entries used by the model.
pub type OverlayEntry = OverlaySelection;

/// Lightweight slot summary used by the overlay renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlaySlotSummary {
    pub slot: u8,
    pub status_label: String,
    pub detail: Option<String>,
}

impl OverlaySlotSummary {
    /// Returns a human-readable suffix for rendering beside slot actions.
    #[must_use]
    pub fn render_suffix(&self) -> String {
        match self.detail.as_deref() {
            Some(detail) if !detail.is_empty() => format!("{} ({detail})", self.status_label),
            _ => self.status_label.clone(),
        }
    }
}

/// Modal pause overlay state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayModel {
    open: bool,
    selected_slot: u8,
    selection_index: usize,
    entries: Vec<OverlayEntry>,
    status_message: Option<String>,
}

impl OverlayModel {
    /// Creates a new overlay model with entries for the given slot count.
    #[must_use]
    pub fn new(slot_count: u8) -> Self {
        let capped_slot_count = slot_count.max(1);
        let mut entries = Vec::with_capacity(usize::from(capped_slot_count) * 2 + 4);
        entries.push(OverlaySelection::Resume);
        entries.push(OverlaySelection::OpenRom);
        for slot in 1..=capped_slot_count {
            entries.push(OverlaySelection::SaveSlot(slot));
        }
        for slot in 1..=capped_slot_count {
            entries.push(OverlaySelection::LoadSlot(slot));
        }
        entries.push(OverlaySelection::Reset);
        entries.push(OverlaySelection::Quit);

        Self {
            open: false,
            selected_slot: 1,
            selection_index: 0,
            entries,
            status_message: None,
        }
    }

    /// Returns whether the overlay is currently open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.open
    }

    /// Opens the overlay.
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Closes the overlay.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Toggles the overlay open state.
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Returns the currently selected entry.
    #[must_use]
    pub fn selection(&self) -> OverlaySelection {
        self.entries[self.selection_index]
    }

    /// Returns all overlay entries in display order.
    #[must_use]
    pub fn entries(&self) -> &[OverlayEntry] {
        &self.entries
    }

    /// Moves selection to the previous entry, wrapping at the top.
    pub fn move_prev(&mut self) {
        if self.selection_index == 0 {
            self.selection_index = self.entries.len().saturating_sub(1);
        } else {
            self.selection_index -= 1;
        }
        self.sync_selected_slot_with_selection();
    }

    /// Moves selection to the next entry, wrapping at the bottom.
    pub fn move_next(&mut self) {
        self.selection_index = (self.selection_index + 1) % self.entries.len();
        self.sync_selected_slot_with_selection();
    }

    /// Activates the current selection and updates overlay-local state.
    #[must_use]
    pub fn activate(&mut self) -> OverlaySelection {
        let selection = self.selection();
        match selection {
            OverlaySelection::SaveSlot(slot) | OverlaySelection::LoadSlot(slot) => {
                self.selected_slot = slot;
            }
            OverlaySelection::Resume
            | OverlaySelection::OpenRom
            | OverlaySelection::Reset
            | OverlaySelection::Quit => {}
        }
        selection
    }

    /// Returns the slot most recently chosen through overlay slot actions.
    #[must_use]
    pub const fn selected_slot(&self) -> u8 {
        self.selected_slot
    }

    /// Repositions the selection to the first entry for the given slot.
    pub fn focus_slot(&mut self, slot: u8, save_action: bool) {
        if let Some(index) = self
            .entries
            .iter()
            .position(|entry| match (save_action, entry) {
                (true, OverlaySelection::SaveSlot(candidate)) => *candidate == slot,
                (false, OverlaySelection::LoadSlot(candidate)) => *candidate == slot,
                _ => false,
            })
        {
            self.selection_index = index;
            self.sync_selected_slot_with_selection();
        }
    }

    /// Sets a user-facing status message shown at the bottom of the overlay.
    pub fn set_status_message(&mut self, message: impl Into<String>) {
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

    fn sync_selected_slot_with_selection(&mut self) {
        match self.selection() {
            OverlaySelection::SaveSlot(slot) | OverlaySelection::LoadSlot(slot) => {
                self.selected_slot = slot;
            }
            OverlaySelection::Resume
            | OverlaySelection::OpenRom
            | OverlaySelection::Reset
            | OverlaySelection::Quit => {}
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
    for ch in text.chars() {
        if ch == '\n' {
            cursor_x = x;
            continue;
        }
        draw_char(frame, frame_width, cursor_x, y, ch, color);
        cursor_x = cursor_x.saturating_add(GLYPH_SIZE + GLYPH_SPACING);
    }
}

/// Draws the full modal overlay on top of the existing frame.
pub fn draw_overlay(
    frame: &mut [u8],
    frame_width: usize,
    frame_height: usize,
    model: &OverlayModel,
    slot_summaries: &[OverlaySlotSummary],
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

    draw_text(frame, frame_width, text_x, text_y, "PAUSED", COLOR_TEXT);
    text_y = text_y.saturating_add(LINE_HEIGHT + 2);
    draw_text(
        frame,
        frame_width,
        text_x,
        text_y,
        "Resume | Open ROM | Save/Load Slots | Reset | Quit",
        COLOR_SUBTEXT,
    );
    text_y = text_y.saturating_add(LINE_HEIGHT + 6);

    for (index, entry) in model.entries().iter().enumerate() {
        let is_selected = index == model.selection_index;
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

        let label = entry_label(*entry, slot_summaries);
        draw_text(
            frame,
            frame_width,
            text_x,
            text_y,
            &label,
            if is_selected {
                COLOR_TEXT
            } else {
                COLOR_SUBTEXT
            },
        );
        text_y = text_y.saturating_add(LINE_HEIGHT);
    }

    if let Some(message) = model.status_message() {
        let footer_y = panel_y
            .saturating_add(panel_height)
            .saturating_sub(PANEL_PADDING + LINE_HEIGHT);
        draw_text(frame, frame_width, text_x, footer_y, message, COLOR_STATUS);
    }
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

fn entry_label(entry: OverlaySelection, slot_summaries: &[OverlaySlotSummary]) -> String {
    match entry {
        OverlaySelection::Resume => "Resume".to_owned(),
        OverlaySelection::OpenRom => "Open ROM...".to_owned(),
        OverlaySelection::SaveSlot(slot) => {
            format!("Save Slot {slot}: {}", slot_suffix(slot, slot_summaries))
        }
        OverlaySelection::LoadSlot(slot) => {
            format!("Load Slot {slot}: {}", slot_suffix(slot, slot_summaries))
        }
        OverlaySelection::Reset => "Reset".to_owned(),
        OverlaySelection::Quit => "Quit".to_owned(),
    }
}

fn slot_suffix(slot: u8, slot_summaries: &[OverlaySlotSummary]) -> String {
    slot_summaries
        .iter()
        .find(|summary| summary.slot == slot)
        .map(OverlaySlotSummary::render_suffix)
        .unwrap_or_else(|| "Unknown".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{OverlayModel, OverlaySelection, OverlaySlotSummary, draw_overlay, draw_text};

    #[test]
    fn overlay_navigation_wraps_and_tracks_selected_slot() {
        let mut overlay = OverlayModel::new(3);
        assert_eq!(overlay.selection(), OverlaySelection::Resume);

        overlay.move_prev();
        assert_eq!(overlay.selection(), OverlaySelection::Quit);

        overlay.move_next();
        assert_eq!(overlay.selection(), OverlaySelection::Resume);

        overlay.move_next();
        assert_eq!(overlay.selection(), OverlaySelection::OpenRom);

        overlay.move_next();
        assert_eq!(overlay.selection(), OverlaySelection::SaveSlot(1));

        let _ = overlay.activate();
        assert_eq!(overlay.selected_slot(), 1);
    }

    #[test]
    fn moving_slot_selection_updates_selected_slot_without_activation() {
        let mut overlay = OverlayModel::new(3);

        overlay.move_next();
        overlay.move_next();
        overlay.move_next();
        assert_eq!(overlay.selection(), OverlaySelection::SaveSlot(2));
        assert_eq!(overlay.selected_slot(), 2);

        overlay.move_next();
        overlay.move_next();
        assert_eq!(overlay.selection(), OverlaySelection::LoadSlot(1));
        assert_eq!(overlay.selected_slot(), 1);
    }

    #[test]
    fn draw_text_marks_pixels_inside_target_buffer() {
        let mut frame = vec![0_u8; 64 * 64 * 4];
        draw_text(&mut frame, 64, 2, 2, "NES", [255, 255, 255, 255]);
        assert!(frame.iter().any(|component| *component != 0));
    }

    #[test]
    fn draw_overlay_renders_panel_selection_and_status_message() {
        let mut frame = vec![16_u8; 128 * 96 * 4];
        let mut overlay = OverlayModel::new(2);
        overlay.open();
        overlay.move_next();
        overlay.set_status_message("Slot 1 loaded");
        let slots = vec![
            OverlaySlotSummary {
                slot: 1,
                status_label: "Saved".to_owned(),
                detail: Some("now".to_owned()),
            },
            OverlaySlotSummary {
                slot: 2,
                status_label: "Empty".to_owned(),
                detail: None,
            },
        ];

        draw_overlay(&mut frame, 128, 96, &overlay, &slots);

        assert!(
            frame.iter().any(|component| *component != 16),
            "overlay draw should mutate the frame"
        );
    }
}
