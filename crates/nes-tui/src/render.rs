//! Terminal rendering engine for the NES emulator.
//!
//! This module provides high-performance routines for downsampling the raw `256x240` RGBA
//! framebuffer from the NES PPU into stylized text structures compatible with the `ratatui`
//! terminal UI library. It supports multiple block-character resolutions (e.g., half-block
//! and quarter-block) to approximate the display in a standard monospaced terminal.

use nes_core::{FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// Downsamples the raw RGBA NES framebuffer into a smaller grid of RGB tuples.
///
/// Uses nearest-neighbor sampling. Useful for building raw color maps or basic
/// single-character (space) rendering modes.
///
/// ## Panics
///
/// This function does not panic, provided the `frame_rgba` matches the expected
/// length of `nes_core::FRAME_RGBA_BYTES`. If the inputs are invalid or target
/// dimensions are zero, it returns an empty vector.
///
/// ## Examples
///
/// ```
/// use nes_core::FRAME_RGBA_BYTES;
/// use nes_tui::render::downsample_frame_rgb;
///
/// let frame = vec![0_u8; FRAME_RGBA_BYTES];
/// // Downsample to a 64x60 grid
/// let cells = downsample_frame_rgb(&frame, 64, 60);
/// assert_eq!(cells.len(), 64 * 60);
/// ```
#[must_use]
pub fn downsample_frame_rgb(
    frame_rgba: &[u8],
    target_width: u16,
    target_height: u16,
) -> Vec<(u8, u8, u8)> {
    if frame_rgba.len() != FRAME_RGBA_BYTES || target_width == 0 || target_height == 0 {
        return Vec::new();
    }

    let width = usize::from(target_width);
    let height = usize::from(target_height);
    assert!(
        width > 0 && height > 0,
        "downsample dimensions must be non-zero after guard validation"
    );
    let mut out = Vec::with_capacity(width * height);

    for y in 0..height {
        let src_y = if height == 1 {
            FRAME_HEIGHT / 2
        } else {
            y * (FRAME_HEIGHT - 1) / (height - 1)
        };
        for x in 0..width {
            let src_x = if width == 1 {
                FRAME_WIDTH / 2
            } else {
                x * (FRAME_WIDTH - 1) / (width - 1)
            };
            let idx = (src_y * FRAME_WIDTH + src_x) * 4;
            out.push((frame_rgba[idx], frame_rgba[idx + 1], frame_rgba[idx + 2]));
        }
    }

    out
}

/// Converts a flat slice of RGB tuples into a list of `ratatui` [`Line`]s using spaces.
///
/// Each cell is rendered as a single space character with the background color
/// set to the RGB tuple. This creates a low-resolution "full block" appearance.
///
/// ## Examples
///
/// ```
/// use nes_tui::render::frame_lines_from_rgb;
///
/// let cells = vec![(255, 0, 0), (0, 255, 0)];
/// let lines = frame_lines_from_rgb(&cells, 2);
/// assert_eq!(lines.len(), 1);
/// assert_eq!(lines[0].spans.len(), 2);
/// ```
#[must_use]
pub fn frame_lines_from_rgb(cells: &[(u8, u8, u8)], width: u16) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }

    let row_width = usize::from(width);
    let mut lines = Vec::with_capacity(cells.len() / row_width);
    for row in cells.chunks(row_width) {
        let mut spans = Vec::with_capacity(row.len());
        for &(r, g, b) in row {
            spans.push(Span::styled(
                " ",
                Style::default()
                    .fg(Color::Rgb(r, g, b))
                    .bg(Color::Rgb(r, g, b)),
            ));
        }
        lines.push(Line::from(spans));
    }
    lines
}

/// Renders the NES framebuffer into `ratatui` lines using Unicode half-blocks (`▀`).
///
/// This technique doubles the vertical resolution in the terminal by using the
/// foreground color for the top half of the character cell and the background color
/// for the bottom half. It averages pixel regions to produce smooth colors.
///
/// ## Examples
///
/// ```
/// use nes_core::FRAME_RGBA_BYTES;
/// use nes_tui::render::frame_lines_half_blocks;
///
/// let frame = vec![0_u8; FRAME_RGBA_BYTES];
/// // Requesting a 64x24 terminal area
/// let lines = frame_lines_half_blocks(&frame, 64, 24);
/// assert_eq!(lines.len(), 24);
/// ```
#[must_use]
pub fn frame_lines_half_blocks(
    frame_rgba: &[u8],
    target_width: u16,
    target_height: u16,
) -> Vec<Line<'static>> {
    if frame_rgba.len() != FRAME_RGBA_BYTES || target_width == 0 || target_height == 0 {
        return Vec::new();
    }

    let width = usize::from(target_width);
    let height = usize::from(target_height);
    let virtual_rows = height.saturating_mul(2);
    let mut lines = Vec::with_capacity(height);

    for row in 0..height {
        let top_virtual_y = row.saturating_mul(2);
        let bottom_virtual_y = top_virtual_y.saturating_add(1);
        let (top_y_start, top_y_end) = bucket_bounds(top_virtual_y, virtual_rows, FRAME_HEIGHT);
        let (bottom_y_start, bottom_y_end) =
            bucket_bounds(bottom_virtual_y, virtual_rows, FRAME_HEIGHT);

        let mut spans = Vec::with_capacity(width);
        for col in 0..width {
            let (x_start, x_end) = bucket_bounds(col, width, FRAME_WIDTH);
            let top = average_region_rgb(frame_rgba, x_start, x_end, top_y_start, top_y_end);
            let bottom =
                average_region_rgb(frame_rgba, x_start, x_end, bottom_y_start, bottom_y_end);
            spans.push(Span::styled(
                "\u{2580}",
                Style::default()
                    .fg(Color::Rgb(top.0, top.1, top.2))
                    .bg(Color::Rgb(bottom.0, bottom.1, bottom.2)),
            ));
        }
        lines.push(Line::from(spans));
    }

    lines
}

/// Renders the NES framebuffer into `ratatui` lines using Unicode quarter-blocks.
///
/// This provides the highest perceived resolution in a terminal by fitting 4 "pixels"
/// (a 2x2 grid) into a single character cell, selecting an optimal foreground, background,
/// and block glyph mask for each cell.
///
/// ## Examples
///
/// ```
/// use nes_core::FRAME_RGBA_BYTES;
/// use nes_tui::render::frame_lines_quarter_blocks;
///
/// let frame = vec![0_u8; FRAME_RGBA_BYTES];
/// let lines = frame_lines_quarter_blocks(&frame, 64, 24);
/// assert_eq!(lines.len(), 24);
/// ```
#[must_use]
pub fn frame_lines_quarter_blocks(
    frame_rgba: &[u8],
    target_width: u16,
    target_height: u16,
) -> Vec<Line<'static>> {
    if frame_rgba.len() != FRAME_RGBA_BYTES || target_width == 0 || target_height == 0 {
        return Vec::new();
    }

    let width = usize::from(target_width);
    let height = usize::from(target_height);
    let virtual_cols = width.saturating_mul(2);
    let virtual_rows = height.saturating_mul(2);
    let mut lines = Vec::with_capacity(height);

    for row in 0..height {
        let top_virtual_y = row.saturating_mul(2);
        let bottom_virtual_y = top_virtual_y.saturating_add(1);
        let src_top_y = map_index(top_virtual_y, virtual_rows, FRAME_HEIGHT);
        let src_bottom_y = map_index(bottom_virtual_y, virtual_rows, FRAME_HEIGHT);

        let mut spans = Vec::with_capacity(width);
        for col in 0..width {
            let left_virtual_x = col.saturating_mul(2);
            let right_virtual_x = left_virtual_x.saturating_add(1);
            let src_left_x = map_index(left_virtual_x, virtual_cols, FRAME_WIDTH);
            let src_right_x = map_index(right_virtual_x, virtual_cols, FRAME_WIDTH);

            let samples = [
                sample_rgb(frame_rgba, src_left_x, src_top_y),
                sample_rgb(frame_rgba, src_right_x, src_top_y),
                sample_rgb(frame_rgba, src_left_x, src_bottom_y),
                sample_rgb(frame_rgba, src_right_x, src_bottom_y),
            ];
            let (mask, fg, bg) = choose_quarter_mask_and_palette(samples);
            spans.push(Span::styled(
                quarter_block_glyph(mask),
                Style::default()
                    .fg(Color::Rgb(fg.0, fg.1, fg.2))
                    .bg(Color::Rgb(bg.0, bg.1, bg.2)),
            ));
        }
        lines.push(Line::from(spans));
    }

    lines
}

/// Extracts a small sequence of color swatches directly from the center of the framebuffer.
///
/// This is used by the TUI to display a quick "color summary" or ambient theme palette
/// based on the current screen contents.
///
/// ## Examples
///
/// ```
/// use nes_core::FRAME_RGBA_BYTES;
/// use nes_tui::render::mini_palette_spans;
///
/// let frame = vec![0_u8; FRAME_RGBA_BYTES];
/// // Extract 5 color swatches
/// let spans = mini_palette_spans(&frame, 5);
/// assert_eq!(spans.len(), 5);
/// ```
#[must_use]
pub fn mini_palette_spans(frame_rgba: &[u8], swatches: usize) -> Vec<Span<'static>> {
    if frame_rgba.len() != FRAME_RGBA_BYTES || swatches == 0 {
        return Vec::new();
    }
    let mut spans = Vec::with_capacity(swatches);
    for idx in 0..swatches {
        let x = map_index(idx, swatches, FRAME_WIDTH);
        let y = FRAME_HEIGHT / 2;
        let (r, g, b) = sample_rgb(frame_rgba, x, y);
        spans.push(Span::styled(
            "  ",
            Style::default()
                .bg(Color::Rgb(r, g, b))
                .fg(Color::Rgb(r, g, b)),
        ));
    }
    spans
}

#[must_use]
fn sample_rgb(frame_rgba: &[u8], x: usize, y: usize) -> (u8, u8, u8) {
    let idx = (y * FRAME_WIDTH + x) * 4;
    (frame_rgba[idx], frame_rgba[idx + 1], frame_rgba[idx + 2])
}

#[must_use]
fn map_index(idx: usize, target_len: usize, source_len: usize) -> usize {
    if source_len <= 1 || target_len <= 1 {
        return 0;
    }
    idx.saturating_mul(source_len - 1) / (target_len - 1)
}

#[must_use]
fn bucket_bounds(idx: usize, bucket_count: usize, source_len: usize) -> (usize, usize) {
    if bucket_count == 0 || source_len == 0 {
        return (0, 0);
    }

    let start = idx.saturating_mul(source_len) / bucket_count;
    let mut end = idx.saturating_add(1).saturating_mul(source_len) / bucket_count;
    if end <= start {
        end = start.saturating_add(1);
    }

    (start.min(source_len.saturating_sub(1)), end.min(source_len))
}

#[must_use]
fn average_region_rgb(
    frame_rgba: &[u8],
    x_start: usize,
    x_end: usize,
    y_start: usize,
    y_end: usize,
) -> (u8, u8, u8) {
    let mut r_sum = 0_u32;
    let mut g_sum = 0_u32;
    let mut b_sum = 0_u32;
    let mut count = 0_u32;

    for y in y_start..y_end {
        for x in x_start..x_end {
            let idx = (y * FRAME_WIDTH + x) * 4;
            r_sum = r_sum.saturating_add(u32::from(frame_rgba[idx]));
            g_sum = g_sum.saturating_add(u32::from(frame_rgba[idx + 1]));
            b_sum = b_sum.saturating_add(u32::from(frame_rgba[idx + 2]));
            count = count.saturating_add(1);
        }
    }

    if count == 0 {
        return sample_rgb(
            frame_rgba,
            x_start.min(FRAME_WIDTH.saturating_sub(1)),
            y_start.min(FRAME_HEIGHT.saturating_sub(1)),
        );
    }

    (
        (r_sum / count) as u8,
        (g_sum / count) as u8,
        (b_sum / count) as u8,
    )
}

#[must_use]
fn choose_quarter_mask_and_palette(samples: [(u8, u8, u8); 4]) -> (u8, (u8, u8, u8), (u8, u8, u8)) {
    let mut candidates = [(0, 0, 0); 4];
    let mut len = 0;
    for sample in samples {
        if !candidates[..len].contains(&sample) {
            candidates[len] = sample;
            len += 1;
        }
    }
    let candidates = &candidates[..len];

    if candidates.is_empty() {
        return (0, (0, 0, 0), (0, 0, 0));
    }
    if candidates.len() == 1 {
        return (15, candidates[0], candidates[0]);
    }

    let mut best_error = u32::MAX;
    let mut best = (15_u8, candidates[0], candidates[1]);

    for &fg in candidates {
        for &bg in candidates {
            let mut mask = 0_u8;
            let mut error = 0_u32;
            for (idx, &sample) in samples.iter().enumerate() {
                let fg_error = rgb_error_sq(sample, fg);
                let bg_error = rgb_error_sq(sample, bg);
                if fg_error <= bg_error {
                    mask |= 1_u8 << idx;
                    error = error.saturating_add(fg_error);
                } else {
                    error = error.saturating_add(bg_error);
                }
            }
            if error < best_error {
                best_error = error;
                best = (mask, fg, bg);
            }
        }
    }

    best
}

#[must_use]
fn rgb_error_sq(a: (u8, u8, u8), b: (u8, u8, u8)) -> u32 {
    let dr = i32::from(a.0) - i32::from(b.0);
    let dg = i32::from(a.1) - i32::from(b.1);
    let db = i32::from(a.2) - i32::from(b.2);
    ((dr * dr) + (dg * dg) + (db * db)) as u32
}

#[must_use]
fn quarter_block_glyph(mask: u8) -> &'static str {
    match mask {
        0 => " ",
        1 => "\u{2598}",  // TL
        2 => "\u{259D}",  // TR
        3 => "\u{2580}",  // TL + TR
        4 => "\u{2596}",  // BL
        5 => "\u{258C}",  // TL + BL
        6 => "\u{259E}",  // TR + BL
        7 => "\u{259B}",  // TL + TR + BL
        8 => "\u{2597}",  // BR
        9 => "\u{259A}",  // TL + BR
        10 => "\u{2590}", // TR + BR
        11 => "\u{259C}", // TL + TR + BR
        12 => "\u{2584}", // BL + BR
        13 => "\u{2599}", // TL + BL + BR
        14 => "\u{259F}", // TR + BL + BR
        _ => "\u{2588}",  // all quadrants
    }
}

#[cfg(test)]
mod tests {
    use super::{
        average_region_rgb, bucket_bounds, downsample_frame_rgb, frame_lines_from_rgb,
        frame_lines_half_blocks, frame_lines_quarter_blocks, map_index, mini_palette_spans,
        quarter_block_glyph, rgb_error_sq,
    };
    use nes_core::{FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH};
    use ratatui::style::Color;

    #[test]
    fn downsample_returns_expected_cell_count() {
        let frame = vec![0_u8; FRAME_RGBA_BYTES];
        let cells = downsample_frame_rgb(&frame, 64, 60);
        assert_eq!(cells.len(), 64 * 60);
    }

    #[test]
    fn downsample_returns_empty_for_invalid_frame_length() {
        let frame = vec![0_u8; FRAME_RGBA_BYTES - 1];
        let cells = downsample_frame_rgb(&frame, 1, 1);
        assert!(cells.is_empty());
    }

    #[test]
    fn downsample_returns_empty_when_target_dimensions_are_zero() {
        let frame = vec![0_u8; FRAME_RGBA_BYTES];
        assert!(downsample_frame_rgb(&frame, 0, 10).is_empty());
        assert!(downsample_frame_rgb(&frame, 10, 0).is_empty());
    }

    #[test]
    fn downsample_uses_nearest_pixel_mapping() {
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];
        write_px(&mut frame, 0, 0, 255, 0, 0);
        write_px(&mut frame, FRAME_WIDTH - 1, 0, 0, 255, 0);
        write_px(&mut frame, 0, FRAME_HEIGHT - 1, 0, 0, 255);
        write_px(&mut frame, FRAME_WIDTH - 1, FRAME_HEIGHT - 1, 255, 255, 0);

        let cells = downsample_frame_rgb(&frame, 2, 2);
        assert_eq!(cells[0], (255, 0, 0));
        assert_eq!(cells[1], (0, 255, 0));
        assert_eq!(cells[2], (0, 0, 255));
        assert_eq!(cells[3], (255, 255, 0));
    }

    #[test]
    fn downsample_height_one_samples_vertical_center_row() {
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];
        let center_y = FRAME_HEIGHT / 2;
        write_px(&mut frame, 0, center_y, 12, 34, 56);
        write_px(&mut frame, FRAME_WIDTH - 1, center_y, 65, 43, 21);

        let cells = downsample_frame_rgb(&frame, 2, 1);
        assert_eq!(cells, vec![(12, 34, 56), (65, 43, 21)]);
    }

    #[test]
    fn downsample_width_one_samples_horizontal_center_column() {
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];
        let center_x = FRAME_WIDTH / 2;
        write_px(&mut frame, center_x, 0, 10, 20, 30);
        write_px(&mut frame, center_x, FRAME_HEIGHT - 1, 210, 220, 230);

        let cells = downsample_frame_rgb(&frame, 1, 2);
        assert_eq!(cells, vec![(10, 20, 30), (210, 220, 230)]);
    }

    #[test]
    fn mini_palette_spans_samples_expected_pixels() {
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];
        let y = FRAME_HEIGHT / 2;
        write_px(&mut frame, 0, y, 10, 11, 12);
        write_px(&mut frame, 85, y, 20, 21, 22);
        write_px(&mut frame, 170, y, 30, 31, 32);
        write_px(&mut frame, FRAME_WIDTH - 1, y, 40, 41, 42);

        let spans = mini_palette_spans(&frame, 4);
        assert_eq!(spans.len(), 4);
        assert_eq!(spans[0].style.bg, Some(Color::Rgb(10, 11, 12)));
        assert_eq!(spans[1].style.bg, Some(Color::Rgb(20, 21, 22)));
        assert_eq!(spans[2].style.bg, Some(Color::Rgb(30, 31, 32)));
        assert_eq!(spans[3].style.bg, Some(Color::Rgb(40, 41, 42)));
    }

    #[test]
    fn mini_palette_spans_supports_single_swatch_without_panicking() {
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];
        let y = FRAME_HEIGHT / 2;
        write_px(&mut frame, 0, y, 77, 88, 99);

        let spans = mini_palette_spans(&frame, 1);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.bg, Some(Color::Rgb(77, 88, 99)));
    }

    #[test]
    fn mini_palette_spans_returns_empty_for_invalid_inputs() {
        assert!(mini_palette_spans(&vec![0_u8; FRAME_RGBA_BYTES - 1], 8).is_empty());
        assert!(mini_palette_spans(&vec![0_u8; FRAME_RGBA_BYTES], 0).is_empty());
    }

    #[test]
    fn map_index_returns_zero_for_degenerate_lengths() {
        assert_eq!(map_index(5, 1, FRAME_WIDTH), 0);
        assert_eq!(map_index(5, 10, 1), 0);
    }

    #[test]
    fn bucket_bounds_handles_zero_bucket_or_source_lengths() {
        assert_eq!(bucket_bounds(0, 0, FRAME_WIDTH), (0, 0));
        assert_eq!(bucket_bounds(0, 4, 0), (0, 0));
    }

    #[test]
    fn bucket_bounds_scales_evenly_for_regular_partitions() {
        assert_eq!(bucket_bounds(1, 4, 16), (4, 8));
        assert_eq!(bucket_bounds(3, 4, 16), (12, 16));
    }

    #[test]
    fn average_region_rgb_uses_independent_rgb_channels() {
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];
        write_px(&mut frame, 0, 0, 10, 200, 30);
        write_px(&mut frame, 1, 0, 40, 50, 60);
        assert_eq!(average_region_rgb(&frame, 0, 2, 0, 1), (25, 125, 45));
    }

    #[test]
    fn rgb_error_sq_matches_expected_distance() {
        assert_eq!(rgb_error_sq((10, 20, 30), (13, 16, 40)), 125);
        assert_eq!(rgb_error_sq((13, 16, 40), (10, 20, 30)), 125);
    }

    #[test]
    fn quarter_block_glyph_maps_all_masks_to_expected_symbols() {
        let expected = [
            " ", "\u{2598}", "\u{259D}", "\u{2580}", "\u{2596}", "\u{258C}", "\u{259E}",
            "\u{259B}", "\u{2597}", "\u{259A}", "\u{2590}", "\u{259C}", "\u{2584}", "\u{2599}",
            "\u{259F}", "\u{2588}",
        ];

        for (mask, glyph) in expected.iter().enumerate() {
            assert_eq!(quarter_block_glyph(mask as u8), *glyph);
        }
    }

    #[test]
    fn frame_lines_match_row_count() {
        let cells = vec![(0_u8, 0_u8, 0_u8); 32 * 16];
        let lines = frame_lines_from_rgb(&cells, 32);
        assert_eq!(lines.len(), 16);
    }

    #[test]
    fn half_block_lines_match_requested_height() {
        let frame = vec![0_u8; FRAME_RGBA_BYTES];
        let lines = frame_lines_half_blocks(&frame, 64, 24);
        assert_eq!(lines.len(), 24);
    }

    #[test]
    fn half_block_lines_return_empty_for_invalid_input_guards() {
        assert!(frame_lines_half_blocks(&vec![0_u8; FRAME_RGBA_BYTES - 1], 8, 8).is_empty());
        assert!(frame_lines_half_blocks(&vec![0_u8; FRAME_RGBA_BYTES], 0, 8).is_empty());
        assert!(frame_lines_half_blocks(&vec![0_u8; FRAME_RGBA_BYTES], 8, 0).is_empty());
    }

    #[test]
    fn half_block_renderer_uses_area_average_for_downsampling() {
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];
        for y in 0..FRAME_HEIGHT {
            for x in 0..FRAME_WIDTH {
                let (r, g, b) = match (x < FRAME_WIDTH / 2, y < FRAME_HEIGHT / 2) {
                    (true, true) => (255, 0, 0),     // top-left
                    (false, true) => (0, 255, 0),    // top-right
                    (true, false) => (0, 0, 255),    // bottom-left
                    (false, false) => (255, 255, 0), // bottom-right
                };
                write_px(&mut frame, x, y, r, g, b);
            }
        }

        let lines = frame_lines_half_blocks(&frame, 1, 1);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 1);
        assert_eq!(lines[0].spans[0].content.as_ref(), "\u{2580}");
        assert_eq!(lines[0].spans[0].style.fg, Some(Color::Rgb(127, 127, 0)));
        assert_eq!(lines[0].spans[0].style.bg, Some(Color::Rgb(127, 127, 127)));
    }

    #[test]
    fn quarter_block_lines_match_requested_height() {
        let frame = vec![0_u8; FRAME_RGBA_BYTES];
        let lines = frame_lines_quarter_blocks(&frame, 64, 24);
        assert_eq!(lines.len(), 24);
    }

    #[test]
    fn quarter_block_lines_return_empty_for_invalid_input_guards() {
        assert!(frame_lines_quarter_blocks(&vec![0_u8; FRAME_RGBA_BYTES - 1], 8, 8).is_empty());
        assert!(frame_lines_quarter_blocks(&vec![0_u8; FRAME_RGBA_BYTES], 0, 8).is_empty());
        assert!(frame_lines_quarter_blocks(&vec![0_u8; FRAME_RGBA_BYTES], 8, 0).is_empty());
    }

    #[test]
    fn quarter_block_preserves_top_bottom_split() {
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];
        write_px(&mut frame, 0, 0, 255, 0, 0);
        write_px(&mut frame, FRAME_WIDTH - 1, 0, 255, 0, 0);
        write_px(&mut frame, 0, FRAME_HEIGHT - 1, 0, 0, 255);
        write_px(&mut frame, FRAME_WIDTH - 1, FRAME_HEIGHT - 1, 0, 0, 255);

        let lines = frame_lines_quarter_blocks(&frame, 1, 1);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 1);
        assert_eq!(lines[0].spans[0].content.as_ref(), "\u{2580}");
        assert_eq!(lines[0].spans[0].style.fg, Some(Color::Rgb(255, 0, 0)));
        assert_eq!(lines[0].spans[0].style.bg, Some(Color::Rgb(0, 0, 255)));
    }

    fn write_px(frame: &mut [u8], x: usize, y: usize, r: u8, g: u8, b: u8) {
        let idx = (y * FRAME_WIDTH + x) * 4;
        frame[idx] = r;
        frame[idx + 1] = g;
        frame[idx + 2] = b;
        frame[idx + 3] = 0xFF;
    }
}
