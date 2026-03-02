use nes_core::{FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

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
    let mut candidates = Vec::with_capacity(4);
    for sample in samples {
        if !candidates.contains(&sample) {
            candidates.push(sample);
        }
    }
    if candidates.is_empty() {
        return (0, (0, 0, 0), (0, 0, 0));
    }
    if candidates.len() == 1 {
        return (15, candidates[0], candidates[0]);
    }

    let mut best_error = u32::MAX;
    let mut best = (15_u8, candidates[0], candidates[1]);

    for &fg in &candidates {
        for &bg in &candidates {
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
        downsample_frame_rgb, frame_lines_from_rgb, frame_lines_half_blocks,
        frame_lines_quarter_blocks,
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
