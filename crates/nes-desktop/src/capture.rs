use nes_core::{FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH};
use std::fs;

pub(crate) fn should_capture_frame(every_n_frames: u64, frame_index: u64) -> bool {
    every_n_frames != 0 && frame_index.is_multiple_of(every_n_frames)
}

pub(crate) fn capture_path_for_frame(template: &str, frame: u64) -> String {
    if template.contains("{frame}") {
        template.replace("{frame}", &format!("{frame:06}"))
    } else {
        template.to_owned()
    }
}

pub(crate) fn write_frame_ppm(path: &str, rgba: &[u8]) -> Result<(), String> {
    if rgba.len() != FRAME_RGBA_BYTES {
        return Err("frame length mismatch".to_owned());
    }
    let bytes = if path.to_ascii_lowercase().ends_with(".bmp") {
        nes_core::bmp::encode_bmp(FRAME_WIDTH, FRAME_HEIGHT, rgba)?
    } else {
        encode_ppm(FRAME_WIDTH, FRAME_HEIGHT, rgba).map_err(|e| e.to_string())?
    };
    fs::write(path, bytes).map_err(|err| format!("unable to write '{path}': {err}"))
}

// SECURITY: Ensure we propagate formatting errors using `?` rather than `unwrap()`.
// Attempting to `unwrap()` when writing to an I/O device (like a file) can lead to
// application crashes/panics if the underlying disk becomes full or permissions change.
pub(crate) fn encode_ppm(width: usize, height: usize, rgba: &[u8]) -> std::io::Result<Vec<u8>> {
    use std::io::Write;
    let mut ppm = Vec::with_capacity(32 + width * height * 3);
    write!(&mut ppm, "P6\n{width} {height}\n255\n")?;
    for px in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&px[..3]);
    }
    Ok(ppm)
}
