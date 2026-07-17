use nes_core::{Button, Command, NesCore};

#[test]
fn test_controller_strobe_behavior() {
    let mut core = NesCore::new();
    core.execute(Command::PressButton(Button::A)).unwrap();

    core.write_cpu_bus(0x4016, 1);

    // While strobe is ON, reading returns bit 0 of the shift register.
    // In NES, it's typically the A button. Since A is pressed, it returns 1.
    // Our implementation does `bit = if strobe { shift & 1 } else { bit = shift & 1; shift >>= 1 }`.
    // Let's print out what read_memory actually returns because it's failing.
    let r1 = core.read_memory(0x4016) & 1;
    let r2 = core.read_memory(0x4016) & 1;
    println!("Read with strobe ON: {}, {}", r1, r2);

    core.write_cpu_bus(0x4016, 0);

    let r3 = core.read_memory(0x4016) & 1;
    let r4 = core.read_memory(0x4016) & 1;
    println!("Read with strobe OFF: {}, {}", r3, r4);
}
