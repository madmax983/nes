#![cfg(feature = "tas")]

use nes_core::{
    Button, Command, NesCore,
    tas::{TasError, TasFrameRun, TasMovie, TasRecorder},
};

fn looping_core() -> NesCore {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]);
    core
}

#[test]
fn tas_recorder_coalesces_identical_frames_into_runs() {
    let mut recorder = TasRecorder::new();
    recorder.start();

    recorder.record_frame(0);
    recorder.record_frame(0);
    recorder.record_frame(Button::A.bit_mask());
    recorder.record_frame(Button::A.bit_mask());
    recorder.record_frame_bits(
        Button::A.bit_mask() | Button::Right.bit_mask(),
        Button::Start.bit_mask(),
    );
    recorder.stop();

    let movie = recorder.movie();
    assert_eq!(
        movie.runs(),
        &[
            TasFrameRun::new(0, 0, 2),
            TasFrameRun::new(Button::A.bit_mask(), 0, 2),
            TasFrameRun::new(
                Button::A.bit_mask() | Button::Right.bit_mask(),
                Button::Start.bit_mask(),
                1,
            ),
        ]
    );
    assert_eq!(movie.total_frames(), 5);
}

#[test]
fn tas_movie_replay_matches_direct_command_execution() {
    let movie = TasMovie::from_runs(vec![
        TasFrameRun::new(Button::Right.bit_mask(), 0, 3),
        TasFrameRun::new(Button::A.bit_mask() | Button::Right.bit_mask(), 0, 2),
        TasFrameRun::new(0, Button::Start.bit_mask(), 1),
    ]);

    let mut expected = looping_core();
    for run in movie.runs() {
        expected
            .execute(Command::SetControllerState(run.controller1_bits))
            .unwrap();
        expected
            .execute(Command::SetController2State(run.controller2_bits))
            .unwrap();
        for _ in 0..run.frames {
            expected.execute(Command::StepFrame).unwrap();
        }
    }

    let mut replayed = looping_core();
    let frames_elapsed = movie.replay(&mut replayed).unwrap();

    assert_eq!(frames_elapsed, movie.total_frames());
    assert_eq!(replayed.state_hash(), expected.state_hash());
    assert_eq!(replayed.controller_bits(), expected.controller_bits());
    assert_eq!(replayed.controller2_bits(), expected.controller2_bits());
}

#[test]
fn tas_movie_exports_legacy_macro_script() {
    let movie = TasMovie::from_runs(vec![
        TasFrameRun::new(0, 0, 3),
        TasFrameRun::new(Button::A.bit_mask(), 0, 2),
        TasFrameRun::new(Button::Right.bit_mask(), 0, 1),
    ]);

    let script = movie.to_macro_script().unwrap();
    assert_eq!(
        script,
        "WAIT 3\nPRESS A\nWAIT 2\nRELEASE A\nPRESS Right\nWAIT 1\n"
    );
}

#[test]
fn tas_movie_macro_export_rejects_player2_input() {
    let movie = TasMovie::from_runs(vec![TasFrameRun::new(0, Button::Start.bit_mask(), 1)]);

    let err = movie.to_macro_script().unwrap_err();
    assert_eq!(err, TasError::Player2MacroScriptUnsupported);
}

#[test]
fn tas_error_display_describes_player2_macro_limit() {
    assert_eq!(
        TasError::Player2MacroScriptUnsupported.to_string(),
        "legacy macro scripts do not support player 2 input"
    );
}

#[test]
fn tas_recorder_start_stop_and_clear_update_observable_state() {
    let mut recorder = TasRecorder::new();
    assert!(!recorder.is_recording());

    recorder.start();
    assert!(recorder.is_recording());

    recorder.record_frame(Button::B.bit_mask());
    assert_eq!(recorder.movie().total_frames(), 1);

    recorder.stop();
    assert!(!recorder.is_recording());

    recorder.record_frame(Button::A.bit_mask());
    assert_eq!(
        recorder.movie().runs(),
        &[TasFrameRun::new(Button::B.bit_mask(), 0, 1)]
    );

    recorder.clear();
    assert!(recorder.movie().runs().is_empty());
    assert_eq!(recorder.movie().total_frames(), 0);
    assert!(!recorder.is_recording());
}

#[test]
fn tas_recorder_can_capture_live_core_state_and_finish_movie() {
    let mut core = looping_core();
    core.execute(Command::SetControllerState(Button::Left.bit_mask()))
        .unwrap();
    core.execute(Command::SetController2State(Button::Select.bit_mask()))
        .unwrap();

    let mut recorder = TasRecorder::new();
    recorder.start();
    recorder.record_core_frame(&core);

    let movie = recorder.finish();
    assert_eq!(
        movie.runs(),
        &[TasFrameRun::new(
            Button::Left.bit_mask(),
            Button::Select.bit_mask(),
            1,
        )]
    );
}

#[test]
fn tas_recorder_macro_script_matches_movie_export() {
    let mut recorder = TasRecorder::new();
    recorder.start();
    recorder.record_frame(0);
    recorder.record_frame(Button::Start.bit_mask());
    recorder.record_frame(Button::Start.bit_mask());
    recorder.stop();

    assert_eq!(
        recorder.macro_script().unwrap(),
        "WAIT 1\nPRESS Start\nWAIT 2\n"
    );
}

#[test]
fn tas_movie_serde_round_trip_preserves_runs() {
    let movie = TasMovie::from_runs(vec![
        TasFrameRun::new(Button::A.bit_mask(), 0, 2),
        TasFrameRun::new(Button::Right.bit_mask(), Button::Start.bit_mask(), 3),
    ]);

    let json = serde_json::to_string(&movie).unwrap();
    let restored: TasMovie = serde_json::from_str(&json).unwrap();

    assert_eq!(restored, movie);
}
