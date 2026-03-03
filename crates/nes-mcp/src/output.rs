use std::cmp;
use std::sync::{Mutex, OnceLock};

use nes_core::{AUDIO_CHUNK_SAMPLES, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH};

const DEFAULT_WIDTH: u32 = FRAME_WIDTH as u32;
const DEFAULT_HEIGHT: u32 = FRAME_HEIGHT as u32;
const DEFAULT_AUDIO_SAMPLE_COUNT: usize = AUDIO_CHUNK_SAMPLES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputMetadata {
    pub frame_seq: u64,
    pub audio_seq: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameChunk {
    pub seq: u64,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioChunk {
    pub seq: u64,
    pub samples: Vec<i16>,
}

#[derive(Debug)]
struct OutputState {
    frame_seq: u64,
    audio_seq: u64,
    width: u32,
    height: u32,
    frame_rgba: Vec<u8>,
    audio_samples: Vec<i16>,
}

impl OutputState {
    fn metadata(&self) -> OutputMetadata {
        OutputMetadata {
            frame_seq: self.frame_seq,
            audio_seq: self.audio_seq,
            width: self.width,
            height: self.height,
        }
    }
}

fn output_state() -> &'static Mutex<OutputState> {
    static STATE: OnceLock<Mutex<OutputState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(OutputState {
            frame_seq: 0,
            audio_seq: 0,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            frame_rgba: vec![0; FRAME_RGBA_BYTES],
            audio_samples: vec![0; DEFAULT_AUDIO_SAMPLE_COUNT],
        })
    })
}

#[must_use]
pub fn latest_output_metadata() -> OutputMetadata {
    output_state().lock().expect("output state lock").metadata()
}

pub fn publish_frame(width: u32, height: u32, rgba: Vec<u8>) {
    let Some(expected_len) = expected_frame_len(width, height) else {
        return;
    };
    if rgba.len() != expected_len {
        return;
    }

    let mut state = output_state().lock().expect("output state lock");
    state.frame_seq = state.frame_seq.saturating_add(1);
    state.width = width;
    state.height = height;
    state.frame_rgba = rgba;
}

pub fn publish_audio(samples: Vec<i16>) {
    let mut state = output_state().lock().expect("output state lock");
    state.audio_seq = state.audio_seq.saturating_add(1);
    state.audio_samples = samples;
}

#[must_use]
pub fn frame_chunk(requested_seq: u64) -> Option<FrameChunk> {
    let mut state = output_state().lock().expect("output state lock");
    state.frame_seq = cmp::max(state.frame_seq, requested_seq);

    Some(FrameChunk {
        seq: state.frame_seq,
        rgba: state.frame_rgba.clone(),
    })
}

#[must_use]
pub fn audio_chunk(requested_seq: u64) -> Option<AudioChunk> {
    let mut state = output_state().lock().expect("output state lock");
    state.audio_seq = cmp::max(state.audio_seq, requested_seq);

    Some(AudioChunk {
        seq: state.audio_seq,
        samples: state.audio_samples.clone(),
    })
}

fn expected_frame_len(width: u32, height: u32) -> Option<usize> {
    let width = usize::try_from(width).ok()?;
    let height = usize::try_from(height).ok()?;
    width.checked_mul(height)?.checked_mul(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expected_frame_len() {
        assert_eq!(expected_frame_len(256, 240), Some(256 * 240 * 4));
        assert_eq!(expected_frame_len(0, 0), Some(0));
        assert_eq!(expected_frame_len(10, 10), Some(400));
    }

    #[test]
    fn test_publish_and_chunk_audio() {
        let initial = latest_output_metadata();
        let samples = vec![42; 735];
        publish_audio(samples.clone());

        let meta = latest_output_metadata();
        assert!(meta.audio_seq > initial.audio_seq);

        let chunk = audio_chunk(meta.audio_seq).unwrap();
        assert_eq!(chunk.seq, meta.audio_seq);
        assert_eq!(chunk.samples, samples);
    }

    #[test]
    fn test_publish_and_chunk_frame() {
        let initial = latest_output_metadata();
        let rgba = vec![128; 256 * 240 * 4];
        publish_frame(256, 240, rgba.clone());

        let meta = latest_output_metadata();
        assert!(meta.frame_seq > initial.frame_seq);
        assert_eq!(meta.width, 256);
        assert_eq!(meta.height, 240);

        let chunk = frame_chunk(meta.frame_seq).unwrap();
        assert_eq!(chunk.seq, meta.frame_seq);
        assert_eq!(chunk.rgba, rgba);
    }

    #[test]
    fn test_publish_frame_invalid_len() {
        let meta_before = latest_output_metadata();
        let invalid_rgba = vec![0; 10]; // Too small
        publish_frame(256, 240, invalid_rgba);

        let meta_after = latest_output_metadata();
        assert_eq!(meta_before.frame_seq, meta_after.frame_seq); // Should not have updated
    }
}
