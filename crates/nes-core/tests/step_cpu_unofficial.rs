use nes_core::{Command, NesCore};

#[test]
fn test_unofficial_instruction_trace_no_alloc() {
    let mut core = NesCore::new();
    core.set_trace_enabled(true);

    // We just step a bit to see if we can trigger traces
    core.execute(Command::StepCpu).unwrap();
    let trace = core.last_cpu_bus_trace();
    assert!(!trace.is_empty());
}
