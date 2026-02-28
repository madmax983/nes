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
fn audio_chunk_is_reset_deterministic() {
    let mut core = NesCore::new();
    let baseline = core.audio_chunk_i16();

    core.execute(Command::Reset).unwrap();
    let after_reset = core.audio_chunk_i16();
    assert_eq!(baseline, after_reset);
}

#[test]
fn audio_chunk_is_input_invariant_without_channel_state_changes() {
    let mut baseline_core = NesCore::new();
    let baseline = baseline_core.audio_chunk_i16();

    let mut with_input_core = NesCore::new();
    with_input_core
        .execute(Command::PressButton(Button::A))
        .unwrap();
    let with_input = with_input_core.audio_chunk_i16();
    assert_eq!(baseline, with_input);
}

#[test]
fn audio_chunk_stays_well_formed_under_long_run() {
    let mut core = NesCore::new();
    for frame in 0..3_600 {
        if frame % 120 == 0 {
            core.execute(Command::PressButton(Button::Right)).unwrap();
        } else if frame % 120 == 80 {
            core.execute(Command::ReleaseButton(Button::Right)).unwrap();
        }
        if frame % 90 == 10 {
            core.execute(Command::PressButton(Button::A)).unwrap();
        } else if frame % 90 == 14 {
            core.execute(Command::ReleaseButton(Button::A)).unwrap();
        }
        core.execute(Command::StepFrame).unwrap();
        let chunk = core.audio_chunk_i16();
        assert_eq!(chunk.len(), AUDIO_CHUNK_SAMPLES, "frame={frame}");
    }
}
