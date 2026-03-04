use nes_core::{Button, Command, NesCore};
use nes_test_harness::build_homebrew_rom;

#[test]
fn generated_homebrew_rom_boots_and_moves_sprite() {
    let rom = build_homebrew_rom().expect("homebrew ROM should build");
    let mut core = NesCore::new();
    core.load_ines_rom(&rom).expect("homebrew ROM should load");

    for _ in 0..30 {
        core.execute(Command::StepFrame)
            .expect("homebrew ROM should boot without stepping errors");
    }

    let x_before = core.ppu_oam_byte(3);
    core.execute(Command::PressButton(Button::Right))
        .expect("press right should succeed");
    for _ in 0..20 {
        core.execute(Command::StepFrame)
            .expect("homebrew ROM should step while right is held");
    }
    core.execute(Command::ReleaseButton(Button::Right))
        .expect("release right should succeed");

    let x_after = core.ppu_oam_byte(3);
    assert!(
        x_after == x_before + 40,
        "expected sprite X position to increase by 40 with Right held (before={x_before}, after={x_after})"
    );
}
