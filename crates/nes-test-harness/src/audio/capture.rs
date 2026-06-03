use nes_core::{Command, CoreError, NesCore};

/// Steps the given [`NesCore`] for a specific number of frames and collects the generated audio samples.
///
/// This is useful for writing test assertions against audio output (e.g., verifying a square wave).
/// Ensure that the [`NesCore`] has a valid ROM loaded prior to calling this; stepping an empty
/// core will result in silent samples.
///
/// # Examples
///
/// ```
/// use nes_core::{NesCore, Command};
/// use nes_test_harness::collect_audio_for_frames;
///
/// let mut core = NesCore::new();
/// // In a real test, load a ROM here before stepping
/// let audio = collect_audio_for_frames(&mut core, 2).unwrap();
/// assert_eq!(audio.len(), nes_core::AUDIO_CHUNK_SAMPLES * 2);
/// ```
pub fn collect_audio_for_frames(core: &mut NesCore, frames: u32) -> Result<Vec<i16>, CoreError> {
    let mut samples = Vec::with_capacity((frames as usize) * nes_core::AUDIO_CHUNK_SAMPLES);
    for _ in 0..frames {
        core.execute(Command::StepFrame)?;
        samples.extend(core.audio_chunk_i16());
    }
    Ok(samples)
}

/// Loads a ROM, advances past an initial warmup period, and captures a window of audio samples.
///
/// This helper handles the full lifecycle for audio testing, including ROM loading and skipping
/// the initial frames (where the game is booting or silent) to capture the target sound.
///
/// # Errors
///
/// Returns an error if the ROM is invalid or if the core fails to step during execution.
///
/// # Examples
///
/// ```
/// use nes_test_harness::{capture_audio_window, build_homebrew_rom};
///
/// let rom_bytes = build_homebrew_rom().unwrap();
/// // Skip the first 10 frames, then capture 5 frames of audio
/// let audio = capture_audio_window(&rom_bytes, 10, 5).unwrap();
/// assert_eq!(audio.len(), nes_core::AUDIO_CHUNK_SAMPLES * 5);
/// ```
pub fn capture_audio_window(
    rom_bytes: &[u8],
    warmup_frames: u32,
    capture_frames: u32,
) -> Result<Vec<i16>, String> {
    let mut core = NesCore::new();
    core.load_ines_rom(rom_bytes)
        .map_err(|err| format!("failed to load ROM for audio capture: {err}"))?;

    for frame in 0..warmup_frames {
        core.execute(Command::StepFrame).map_err(|err| {
            format!(
                "audio warmup frame step failed at frame {frame}, pc={:04X}: {err}",
                core.cpu_pc()
            )
        })?;
    }
    collect_audio_for_frames(&mut core, capture_frames)
        .map_err(|err| format!("audio capture failed after warmup: {err}"))
}
