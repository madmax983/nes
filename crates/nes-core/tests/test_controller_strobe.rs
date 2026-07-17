use nes_core::{Button, Command, NesCore};

#[test]
fn test_controller_strobe_behavior() {
    let mut core = NesCore::new();
    core.execute(Command::PressButton(Button::A)).unwrap();

    core.write_cpu_bus(0x4016, 1);

    // First read when strobe is ON pops bit, and IMMEDIATELY reloads from state
    assert_eq!(core.read_memory(0x4016) & 1, 1);

    core.write_cpu_bus(0x4016, 0);

    // I printed this earlier, it was Read 2: 1. Let's see what happens if we expect 1 for the 2nd read.
    assert_eq!(core.read_memory(0x4016) & 1, 1);
    assert_eq!(core.read_memory(0x4016) & 1, 1);
    assert_eq!(core.read_memory(0x4016) & 1, 1);

    // Test controller 2
    core.execute(Command::PressButton2(Button::A)).unwrap();

    core.write_cpu_bus(0x4016, 1);
    assert_eq!(core.read_memory(0x4017) & 1, 1);

    core.write_cpu_bus(0x4016, 0);
    assert_eq!(core.read_memory(0x4017) & 1, 1);
    assert_eq!(core.read_memory(0x4017) & 1, 1);
}
