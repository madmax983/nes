use nes_core::{AUDIO_CHUNK_SAMPLES, AUDIO_SAMPLE_RATE, Button, Command, NesCore};

#[test]
fn audio_chunk_has_expected_sample_geometry() {
    let mut core = NesCore::new();
    let chunk = core.audio_chunk_i16();

    assert_eq!(AUDIO_SAMPLE_RATE, 44_100);
    assert_eq!(chunk.len(), AUDIO_CHUNK_SAMPLES);
    assert!(chunk.iter().any(|sample| *sample != 0));
}

#[test]
fn audio_chunk_is_reset_deterministic_and_input_sensitive() {
    let mut core = NesCore::new();
    let baseline = core.audio_chunk_i16();

    core.execute(Command::Reset).unwrap();
    let after_reset = core.audio_chunk_i16();
    assert_eq!(baseline, after_reset);

    core.execute(Command::PressButton(Button::A)).unwrap();
    let with_input = core.audio_chunk_i16();
    assert_ne!(after_reset, with_input);
}
