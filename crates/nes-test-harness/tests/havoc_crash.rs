use nes_core::Command;
use nes_core::NesCore;

#[test]
#[should_panic(expected = "index out of bounds")]
fn test_ppu_crash() {
    let mut core = NesCore::new();
    core.execute(Command::StepCpu).unwrap();
    core.write_cpu_bus(10213, 57);
    let _ = core.save_state();
    core.write_cpu_bus(14701, 77);
    let _ = core.save_state();
    core.write_cpu_bus(14097, 62);
    let _ = core.save_state();
    core.execute(Command::StepFrame).unwrap();
    let _ = core.save_state();
    core.write_cpu_bus(11463, 121);
}
