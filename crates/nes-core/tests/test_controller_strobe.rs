use nes_core::{Button, Command, NesCore};

#[test]
fn test_controller_strobe_behavior() {
    let mut core = NesCore::new();
    core.write_cpu_bus(0x4016, 1);
    core.execute(Command::PressButton(Button::A)).unwrap();
    assert_eq!(core.controller_bits(), Button::A.bit_mask());

    // First read when strobe is ON pops bit, and IMMEDIATELY reloads from state
    assert_eq!(core.read_memory(0x4016) & 1, 1);
    assert_eq!(core.read_memory(0x4016) & 1, 1);

    // Turn strobe off
    core.write_cpu_bus(0x4016, 0);

    // When strobe is OFF, the shift register contains the currently held bits.
    // Button A is held, which is bit 0, so the first read is 1.
    assert_eq!(core.read_memory(0x4016) & 1, 1);

    // Test controller 2
    core.write_cpu_bus(0x4016, 1);
    core.execute(Command::PressButton2(Button::A)).unwrap();
    assert_eq!(core.read_memory(0x4017) & 1, 1);
    assert_eq!(core.read_memory(0x4017) & 1, 1);
    core.write_cpu_bus(0x4016, 0);
    assert_eq!(core.read_memory(0x4017) & 1, 1);
}
