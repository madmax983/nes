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
        let src_top_y = map_index(top_virtual_y, virtual_rows, FRAME_HEIGHT);
        let src_bottom_y = map_index(bottom_virtual_y, virtual_rows, FRAME_HEIGHT);

        let mut spans = Vec::with_capacity(width);
        for col in 0..width {
            let src_x = map_index(col, width, FRAME_WIDTH);
            let top = sample_rgb(frame_rgba, src_x, src_top_y);
            let bottom = sample_rgb(frame_rgba, src_x, src_bottom_y);
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

#[cfg(test)]
mod tests {
    use super::{downsample_frame_rgb, frame_lines_from_rgb, frame_lines_half_blocks};
    use nes_core::{FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH};

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

    fn write_px(frame: &mut [u8], x: usize, y: usize, r: u8, g: u8, b: u8) {
        let idx = (y * FRAME_WIDTH + x) * 4;
        frame[idx] = r;
        frame[idx + 1] = g;
        frame[idx + 2] = b;
        frame[idx + 3] = 0xFF;
    }
}
