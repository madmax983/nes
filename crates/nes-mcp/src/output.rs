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
