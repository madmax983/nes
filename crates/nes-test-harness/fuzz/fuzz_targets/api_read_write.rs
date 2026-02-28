#![no_main]
use libfuzzer_sys::fuzz_target;
use nes_core::NesCore;
use arbitrary::Arbitrary;

#[derive(Arbitrary, Debug)]
enum FuzzAction {
    ReadMemory(u16),
    WriteCpuBus(u16, u8),
    ReadApuStatus,
    CpuA,
    CpuX,
    CpuPc,
}

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    actions: Vec<FuzzAction>,
}

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = FuzzInput::arbitrary(&mut arbitrary::Unstructured::new(data)) {
        let mut core = NesCore::new();
        for action in input.actions {
            match action {
                FuzzAction::ReadMemory(addr) => { let _ = core.read_memory(addr); }
                FuzzAction::WriteCpuBus(addr, val) => { core.write_cpu_bus(addr, val); }
                FuzzAction::ReadApuStatus => { let _ = core.read_apu_status(); }
                FuzzAction::CpuA => { let _ = core.cpu_a(); }
                FuzzAction::CpuX => { let _ = core.cpu_x(); }
                FuzzAction::CpuPc => { let _ = core.cpu_pc(); }
            }
        }
    }
});
