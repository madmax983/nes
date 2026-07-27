import re

with open('crates/nes-core/src/constants.rs', 'r') as f:
    content = f.read()

tests = """
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_rgba_bytes_is_exact_buffer_size() {
        // Assert the exact byte size needed for a 256x240 RGBA framebuffer
        assert_eq!(FRAME_RGBA_BYTES, 245_760);
    }

    #[test]
    fn test_audio_chunk_samples_is_correct_for_60hz() {
        // Assert the exact number of samples for 44.1kHz at 60 FPS
        assert_eq!(AUDIO_CHUNK_SAMPLES, 735);
    }
}
"""

if '#[cfg(test)]' not in content:
    with open('crates/nes-core/src/constants.rs', 'a') as f:
        f.write('\n' + tests)
