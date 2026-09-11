//! Host-side tests for the `no_std` [`nes_embassy::EmuDriver`].
//!
//! These run the exact driver firmware uses (same code, `std` test harness)
//! through 60 emulated frames and check the video/audio/controller plumbing.

use nes_core::{CoreQuery, QueryResult, AUDIO_CHUNK_SAMPLES, FRAME_HEIGHT, FRAME_WIDTH};
use nes_embassy::{buttons, Button, EmuDriver};

/// Builds a minimal NROM iNES image: 16KB PRG with a `NOP; JMP self` spin
/// loop at the reset vector, 8KB of CHR.
fn nrom_spin_rom() -> Vec<u8> {
    let prg_len = 16 * 1024;
    let mut rom = vec![0_u8; 16 + prg_len + 8 * 1024];
    rom[0..4].copy_from_slice(b"NES\x1A");
    rom[4] = 1; // 1 x 16KB PRG
    rom[5] = 1; // 1 x 8KB CHR
                // PRG: NOP ; JMP $C000 at the start of the bank.
    rom[16] = 0xEA;
    rom[17] = 0x4C;
    rom[18] = 0x00;
    rom[19] = 0xC0;
    // Reset vector -> $C000.
    rom[16 + prg_len - 4] = 0x00;
    rom[16 + prg_len - 3] = 0xC0;
    rom
}

fn loaded_driver() -> EmuDriver {
    let mut driver = EmuDriver::new();
    driver.load_rom(&nrom_spin_rom()).unwrap();
    driver
}

#[test]
fn driver_steps_sixty_frames_with_sane_rgb565_output() {
    let mut driver = loaded_driver();
    for _ in 0..60 {
        driver.step_frame().unwrap();
    }

    let frame = driver.frame_rgb565();
    assert_eq!(frame.len(), FRAME_WIDTH * FRAME_HEIGHT);
    assert_eq!(FRAME_WIDTH, 256);
    assert_eq!(FRAME_HEIGHT, 240);
    // A powered-on PPU renders the backdrop color, not black.
    assert!(
        frame.iter().any(|&px| px != 0),
        "expected non-black pixels after 60 frames"
    );
}

#[test]
fn driver_frame_matches_core_rgb565_capture() {
    let mut driver = loaded_driver();
    driver.step_frame().unwrap();

    // The driver's captured frame must equal a direct core capture: the
    // driver adds no transformation between the core and the display.
    let mut direct = vec![0_u16; FRAME_WIDTH * FRAME_HEIGHT];
    driver.core().fill_framebuffer_rgb565(&mut direct);
    assert_eq!(driver.frame_rgb565(), direct.as_slice());
}

#[test]
fn controller_bitfields_reach_the_core() {
    let mut driver = loaded_driver();
    driver.set_controllers(buttons(&[Button::A, Button::Start]), buttons(&[Button::Up]));
    driver.step_frame().unwrap();

    let state = match driver.core().query(CoreQuery::EmulatorState) {
        QueryResult::EmulatorState(state) => state,
        other => panic!("unexpected query result: {other:?}"),
    };
    assert_eq!(
        state.controller_bits,
        Button::A.bit_mask() | Button::Start.bit_mask()
    );
    assert_eq!(state.controller2_bits, Button::Up.bit_mask());

    // Releasing the buttons clears the latched state on the next frame.
    driver.set_controllers(0, 0);
    driver.step_frame().unwrap();
    let state = match driver.core().query(CoreQuery::EmulatorState) {
        QueryResult::EmulatorState(state) => state,
        other => panic!("unexpected query result: {other:?}"),
    };
    assert_eq!(state.controller_bits, 0);
    assert_eq!(state.controller2_bits, 0);
}

#[test]
fn audio_chunk_has_expected_shape_and_is_deterministic() {
    let mut first = loaded_driver();
    let mut second = loaded_driver();
    for _ in 0..60 {
        first.step_frame().unwrap();
        second.step_frame().unwrap();
        assert_eq!(first.audio_chunk().len(), AUDIO_CHUNK_SAMPLES);
        assert_eq!(AUDIO_CHUNK_SAMPLES, 735);
    }
    // Same ROM + same inputs => identical audio and video: no hidden state.
    assert_eq!(first.audio_chunk(), second.audio_chunk());
    assert_eq!(first.frame_rgb565(), second.frame_rgb565());
}

#[test]
fn buttons_helper_packs_bits_in_core_order() {
    assert_eq!(buttons(&[]), 0);
    assert_eq!(buttons(&[Button::A]), 0b0000_0001);
    assert_eq!(buttons(&[Button::Right]), 0b1000_0000);
    assert_eq!(
        buttons(&[Button::B, Button::Select, Button::Down, Button::Left]),
        0b0110_0110
    );
}
