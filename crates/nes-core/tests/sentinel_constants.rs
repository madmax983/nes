use nes_core::{AUDIO_CHUNK_SAMPLES, AUDIO_SAMPLE_RATE};

#[test]
fn test_audio_chunk_samples_exact() {
    assert_eq!(AUDIO_CHUNK_SAMPLES, (AUDIO_SAMPLE_RATE as usize) / 60);
    assert_eq!(AUDIO_CHUNK_SAMPLES, 44100 / 60);
    assert_eq!(AUDIO_CHUNK_SAMPLES, 735);
}
