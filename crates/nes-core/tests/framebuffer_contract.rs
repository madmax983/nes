use nes_core::{Command, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH, NesCore};

#[test]
fn framebuffer_geometry_matches_nes_resolution() {
    let core = NesCore::new();
    let frame = core.framebuffer_rgba();

    assert_eq!(FRAME_WIDTH, 256);
    assert_eq!(FRAME_HEIGHT, 240);
    assert_eq!(FRAME_RGBA_BYTES, FRAME_WIDTH * FRAME_HEIGHT * 4);
    assert_eq!(frame.len(), FRAME_RGBA_BYTES);
    assert!(frame.chunks_exact(4).all(|px| px[3] == 0xFF));
}

#[test]
fn framebuffer_changes_after_frame_step() {
    let mut core = NesCore::new();
    let before = core.framebuffer_rgba();

    core.execute(Command::StepFrame).unwrap();
    core.execute(Command::StepFrame).unwrap();
    let after = core.framebuffer_rgba();

    assert_ne!(before, after);
}
