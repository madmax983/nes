use std::cmp;
use std::sync::{Mutex, OnceLock};

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
            width: 256,
            height: 240,
        })
    })
}

#[must_use]
pub fn latest_output_metadata() -> OutputMetadata {
    output_state().lock().expect("output state lock").metadata()
}

#[must_use]
pub fn frame_chunk(requested_seq: u64) -> Option<FrameChunk> {
    let mut state = output_state().lock().expect("output state lock");
    state.frame_seq = cmp::max(state.frame_seq, requested_seq);

    Some(FrameChunk {
        seq: state.frame_seq,
        rgba: vec![0; 16],
    })
}

#[must_use]
pub fn audio_chunk(requested_seq: u64) -> Option<AudioChunk> {
    let mut state = output_state().lock().expect("output state lock");
    state.audio_seq = cmp::max(state.audio_seq, requested_seq);

    Some(AudioChunk {
        seq: state.audio_seq,
        samples: vec![0; 16],
    })
}
