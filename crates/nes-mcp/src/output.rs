//! Manages shared audio and video output state for the MCP server.
//!
//! Because the emulator generates full 256x240 RGBA frames (~245KB each) and
//! audio samples 60 times a second, copying these buffers for every single JSON
//! RPC query would cause severe memory pressure. This module uses `Arc` wrapped
//! data structures to allow multiple readers to hold references to the latest
//! output chunk without needing to deep-copy the underlying arrays.

use std::cmp;
use std::sync::{Arc, Mutex, OnceLock};

use nes_core::{AUDIO_CHUNK_SAMPLES, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH};

const DEFAULT_WIDTH: u32 = FRAME_WIDTH as u32;
const DEFAULT_HEIGHT: u32 = FRAME_HEIGHT as u32;
const DEFAULT_AUDIO_SAMPLE_COUNT: usize = AUDIO_CHUNK_SAMPLES;

/// Current metadata snapshot of the publisher's output state.
///
/// This metadata allows consumers to quickly check if new frames or audio
/// samples have been published by the emulator without needing to acquire locks
/// or perform deep copies of the underlying media buffers.
///
/// ## Examples
///
/// ```
/// use nes_mcp::OutputMetadata;
///
/// // Metadata is typically retrieved via `latest_output_metadata()`
/// let metadata = OutputMetadata {
///     frame_seq: 42,
///     audio_seq: 15,
///     width: 256,
///     height: 240,
/// };
///
/// if metadata.frame_seq > 0 {
///     println!("The emulator has rendered {} frames!", metadata.frame_seq);
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputMetadata {
    /// Incremented each time a new frame is published.
    pub frame_seq: u64,
    /// Incremented each time a new audio chunk is published.
    pub audio_seq: u64,
    /// The width of the current published frame.
    pub width: u32,
    /// The height of the current published frame.
    pub height: u32,
}

/// A reference-counted video frame snapshot.
///
/// By utilizing `Arc<Vec<u8>>`, the MCP server can broadcast a single rendered
/// frame to multiple connected clients (or tool invocations) simultaneously.
/// This zero-copy approach prevents severe memory pressure when dealing with
/// the ~245KB RGBA payload of a standard NES frame.
///
/// ## Examples
///
/// ```
/// use std::sync::Arc;
/// use nes_mcp::FrameChunk;
///
/// // A blank, black frame ready to be shipped over JSON-RPC.
/// let chunk = FrameChunk {
///     seq: 1,
///     rgba: Arc::new(vec![0; 256 * 240 * 4]),
/// };
///
/// assert_eq!(chunk.rgba.len(), 245760);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameChunk {
    /// The sequence number of this frame.
    pub seq: u64,
    /// We use `Arc<Vec<u8>>` instead of `Vec<u8>` here to allow the MCP engine to
    /// share a single read-only view of the framebuffer across threads without
    /// performing a deep copy (approx ~245KB allocation) every time it's queried.
    pub rgba: Arc<Vec<u8>>,
}

/// A reference-counted block of audio samples.
///
/// Similar to `FrameChunk`, this structure wraps raw `i16` audio samples in
/// an `Arc`. This ensures that high-frequency audio polling from external
/// tools does not trigger expensive memory allocations on the hot path.
///
/// ## Examples
///
/// ```
/// use std::sync::Arc;
/// use nes_mcp::AudioChunk;
///
/// // A block of pure silence, perhaps from a paused emulator.
/// let chunk = AudioChunk {
///     seq: 1,
///     samples: Arc::new(vec![0; 735]),
/// };
///
/// assert_eq!(chunk.samples.len(), 735);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioChunk {
    /// The sequence number of this audio chunk.
    pub seq: u64,
    /// We use `Arc<Vec<i16>>` to share audio samples safely without incurring
    /// per-query memory allocations or deep `.clone()` operations on the hot path.
    pub samples: Arc<Vec<i16>>,
}

#[derive(Debug)]
struct OutputState {
    frame_seq: u64,
    audio_seq: u64,
    width: u32,
    height: u32,
    frame_rgba: Arc<Vec<u8>>,
    audio_samples: Arc<Vec<i16>>,
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
            frame_rgba: Arc::new(vec![0; FRAME_RGBA_BYTES]),
            audio_samples: Arc::new(vec![0; DEFAULT_AUDIO_SAMPLE_COUNT]),
        })
    })
}

/// Returns the current metadata for available frame and audio chunks.
///
/// ## Examples
///
/// ```
/// use nes_mcp::latest_output_metadata;
///
/// let meta = latest_output_metadata();
/// assert_eq!(meta.width, 256);
/// assert_eq!(meta.height, 240);
/// ```
#[must_use]
pub fn latest_output_metadata() -> OutputMetadata {
    output_state().lock().expect("output state lock").metadata()
}

/// Updates the globally shared output state with a new video frame.
///
/// This increments the internal `frame_seq` counter and wraps the payload
/// in an `Arc` for lock-free reader access.
///
/// ## Examples
///
/// ```
/// use nes_mcp::{publish_frame, latest_output_metadata};
///
/// let empty_frame = vec![0; 256 * 240 * 4];
/// publish_frame(256, 240, empty_frame);
///
/// let meta = latest_output_metadata();
/// assert!(meta.frame_seq > 0);
/// ```
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
    state.frame_rgba = Arc::new(rgba);
}

/// Updates the globally shared output state with a new video frame by modifying the existing allocation.
pub fn publish_frame_with<F>(width: u32, height: u32, f: F)
where
    F: FnOnce(&mut [u8]),
{
    let Some(expected_len) = expected_frame_len(width, height) else {
        return;
    };

    let mut state = output_state().lock().expect("output state lock");
    let buffer = Arc::make_mut(&mut state.frame_rgba);

    if buffer.len() != expected_len {
        buffer.resize(expected_len, 0);
    }

    f(buffer);

    state.frame_seq = state.frame_seq.saturating_add(1);
    state.width = width;
    state.height = height;
}

/// Updates the globally shared output state with a new audio chunk.
///
/// This increments the internal `audio_seq` counter and wraps the samples
/// in an `Arc` for lock-free reader access.
///
/// ## Examples
///
/// ```
/// use nes_mcp::{publish_audio, latest_output_metadata};
///
/// publish_audio(vec![0; 735]);
///
/// let meta = latest_output_metadata();
/// assert!(meta.audio_seq > 0);
/// ```
pub fn publish_audio(samples: Vec<i16>) {
    let mut state = output_state().lock().expect("output state lock");
    state.audio_seq = state.audio_seq.saturating_add(1);
    state.audio_samples = Arc::new(samples);
}

/// Updates the globally shared output state with a new audio chunk by modifying the existing allocation.
pub fn publish_audio_with<F>(len: usize, f: F)
where
    F: FnOnce(&mut [i16]),
{
    let mut state = output_state().lock().expect("output state lock");
    let buffer = Arc::make_mut(&mut state.audio_samples);

    if buffer.len() != len {
        buffer.resize(len, 0);
    }

    f(buffer);

    state.audio_seq = state.audio_seq.saturating_add(1);
}

/// Retrieves a reference-counted view of the requested frame sequence.
///
/// If the requested sequence is newer than the current sequence, the state
/// acts as a fast-forward and clamps to the requested sequence to prevent
/// future drift on the client side.
///
/// ## Examples
///
/// ```
/// use nes_mcp::{frame_chunk, publish_frame};
///
/// publish_frame(256, 240, vec![0; 256 * 240 * 4]);
/// let chunk = frame_chunk(1).expect("failed");
/// assert_eq!(chunk.rgba.len(), 256 * 240 * 4);
/// ```
#[must_use]
pub fn frame_chunk(requested_seq: u64) -> Option<FrameChunk> {
    let mut state = output_state().lock().expect("output state lock");
    state.frame_seq = cmp::max(state.frame_seq, requested_seq);

    Some(FrameChunk {
        seq: state.frame_seq,
        rgba: state.frame_rgba.clone(),
    })
}

/// Retrieves a reference-counted view of the requested audio sequence.
///
/// If the requested sequence is newer than the current sequence, the state
/// acts as a fast-forward and clamps to the requested sequence to prevent
/// future drift on the client side.
///
/// ## Examples
///
/// ```
/// use nes_mcp::{audio_chunk, publish_audio};
///
/// publish_audio(vec![0; 735]);
/// let chunk = audio_chunk(1).expect("failed");
/// assert_eq!(chunk.samples.len(), 735);
/// ```
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
pub(crate) fn reset_output_state_for_test() -> std::sync::MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let guard = TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("output test lock");

    let mut state = output_state().lock().expect("output state lock");
    *state = OutputState {
        frame_seq: 0,
        audio_seq: 0,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        frame_rgba: Arc::new(vec![0; FRAME_RGBA_BYTES]),
        audio_samples: Arc::new(vec![0; DEFAULT_AUDIO_SAMPLE_COUNT]),
    };
    drop(state);

    guard
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_none_when_expected_frame_len_overflows() {
        let _guard = reset_output_state_for_test();
        assert_eq!(
            expected_frame_len(u32::MAX, u32::MAX),
            None,
            "Expected None due to width * height overflow"
        );
        // On 32-bit platforms, an overflow happens earlier (width * height),
        // whereas on 64-bit platforms it happens later (during * 4).
    }

    #[test]
    fn should_calculate_correct_frame_length_for_valid_dimensions() {
        let _guard = reset_output_state_for_test();
        assert_eq!(
            expected_frame_len(256, 240),
            Some(245_760),
            "Expected 256 * 240 * 4 = 245760"
        );
    }

    #[test]
    fn should_ignore_publish_frame_if_rgba_length_is_invalid() {
        let _guard = reset_output_state_for_test();
        let initial_meta = latest_output_metadata();

        // Pass an array that is definitely wrong sized (1 byte)
        publish_frame(256, 240, vec![0]);

        let final_meta = latest_output_metadata();
        assert_eq!(
            initial_meta.frame_seq, final_meta.frame_seq,
            "Frame sequence should not increment for invalid frame lengths"
        );
    }

    #[test]
    fn should_fast_forward_frame_chunk_sequence_when_requested_seq_is_newer() {
        let _guard = reset_output_state_for_test();
        let initial_meta = latest_output_metadata();
        let future_seq = initial_meta.frame_seq + 10;

        let chunk = frame_chunk(future_seq).expect("Frame chunk must exist");
        assert_eq!(
            chunk.seq, future_seq,
            "Frame chunk sequence should fast-forward to the requested future sequence"
        );

        let final_meta = latest_output_metadata();
        assert_eq!(
            final_meta.frame_seq, future_seq,
            "Global metadata frame sequence should be fast-forwarded"
        );
    }

    #[test]
    fn should_fast_forward_audio_chunk_sequence_when_requested_seq_is_newer() {
        let _guard = reset_output_state_for_test();
        let initial_meta = latest_output_metadata();
        let future_seq = initial_meta.audio_seq + 5;

        let chunk = audio_chunk(future_seq).expect("Audio chunk must exist");
        assert_eq!(
            chunk.seq, future_seq,
            "Audio chunk sequence should fast-forward to the requested future sequence"
        );

        let final_meta = latest_output_metadata();
        assert_eq!(
            final_meta.audio_seq, future_seq,
            "Global metadata audio sequence should be fast-forwarded"
        );
    }

    #[test]
    fn should_reuse_memory_when_publishing_frame_with_closure() {
        let _guard = reset_output_state_for_test();
        let initial_meta = latest_output_metadata();
        publish_frame_with(256, 240, |buf| {
            assert_eq!(buf.len(), 245_760);
            buf[0] = 99;
        });
        let meta = latest_output_metadata();
        assert_eq!(meta.frame_seq, initial_meta.frame_seq + 1);
        let chunk = frame_chunk(meta.frame_seq).expect("failed");
        assert_eq!(chunk.rgba[0], 99);
    }

    #[test]
    fn should_reuse_memory_when_publishing_audio_with_closure() {
        let _guard = reset_output_state_for_test();
        let initial_meta = latest_output_metadata();
        publish_audio_with(735, |buf| {
            assert_eq!(buf.len(), 735);
            buf[0] = 42;
        });
        let meta = latest_output_metadata();
        assert_eq!(meta.audio_seq, initial_meta.audio_seq + 1);
        let chunk = audio_chunk(meta.audio_seq).expect("failed");
        assert_eq!(chunk.samples[0], 42);
    }

    #[test]
    fn should_resize_memory_when_publishing_audio_with_closure_if_length_differs() {
        let _guard = reset_output_state_for_test();
        publish_audio(vec![0; 100]);
        publish_audio_with(200, |samples| {
            assert_eq!(samples.len(), 200);
        });
    }

    #[test]
    fn should_resize_memory_when_publishing_frame_with_closure_if_length_differs() {
        let _guard = reset_output_state_for_test();
        publish_frame(256, 240, vec![0; 256 * 240 * 4]);
        publish_frame_with(128, 120, |rgba| {
            assert_eq!(rgba.len(), 128 * 120 * 4);
        });
    }

    #[test]
    fn should_increment_audio_seq_and_update_samples_on_publish_audio() {
        let _guard = reset_output_state_for_test();
        let initial_meta = latest_output_metadata();

        publish_audio(vec![42; 735]);

        let final_meta = latest_output_metadata();
        assert_eq!(
            final_meta.audio_seq,
            initial_meta.audio_seq + 1,
            "Audio sequence should increment by 1"
        );

        let chunk = audio_chunk(final_meta.audio_seq).expect("Audio chunk must exist");
        assert_eq!(
            chunk.samples.len(),
            735,
            "Audio chunk should contain the newly published samples"
        );
        assert_eq!(
            chunk.samples[0], 42,
            "Sample content should match what was published"
        );
    }
}
