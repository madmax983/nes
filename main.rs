use nes_core::{NesCore, Command, Button};

fn main() {
    let mut core = NesCore::new();
    // Load a minimal dummy ROM (normally you'd load a real .nes file)
    let mut dummy_rom = vec![
        0x4E, 0x45, 0x53, 0x1A, // "NES\x1A"
        0x01, 0x01, 0x00, 0x00, // 16KB PRG, 8KB CHR, NROM
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Padding
    ];
    // Append 16KB PRG ROM and 8KB CHR ROM to make it a valid cartridge
    dummy_rom.extend(vec![0x00; 16 * 1024 + 8 * 1024]);
    core.load_ines_rom(&dummy_rom).unwrap();

    // Execute commands to drive the core
    core.execute(Command::StepFrame).unwrap();
    core.execute(Command::PressButton(Button::A)).unwrap();

    // Extract framebuffer for rendering
    let frame = core.framebuffer_rgba();
    assert_eq!(frame.len(), 256 * 240 * 4);
}