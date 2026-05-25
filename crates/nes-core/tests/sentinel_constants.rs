use nes_core::constants::*;

#[test]
fn test_audio_chunk_samples() {
    assert_eq!(AUDIO_CHUNK_SAMPLES, (AUDIO_SAMPLE_RATE as usize) / 60);
    assert_eq!(AUDIO_CHUNK_SAMPLES, 735);
}
