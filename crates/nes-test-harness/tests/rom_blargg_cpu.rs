use std::collections::VecDeque;
use std::fs;

use nes_core::{Command, NesCore};
use nes_test_harness::blargg_cpu_rom_path;

const BLARGG_STATUS_ADDR: u16 = 0x6000;
const BLARGG_MESSAGE_ADDR: u16 = 0x6004;
const BLARGG_MAX_STEPS: u32 = 5_000_000;
const BLARGG_MIN_COMPLETION_STEP: u32 = 250_000;

fn read_blargg_message(core: &NesCore) -> String {
    let mut bytes = Vec::new();
    for offset in 0..256_u16 {
        let value = core.read_memory(BLARGG_MESSAGE_ADDR.wrapping_add(offset));
        if value == 0 {
            break;
        }
        bytes.push(value);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

#[test]
#[ignore = "requires roms.blargg_cpu in nes.toml"]
fn blargg_cpu_rom_reports_pass() {
    let rom_path = blargg_cpu_rom_path();

    let bytes = fs::read(&rom_path).expect("failed to read blargg cpu ROM");
    let mut core = NesCore::new();
    core.load_ines_rom(&bytes)
        .expect("failed to load blargg cpu ROM");

    let mut completed = false;
    let mut saw_running_state = false;
    let mut recent_traces = VecDeque::with_capacity(24);
    for step in 0..BLARGG_MAX_STEPS {
        match core.execute(Command::StepCpu) {
            Ok(()) => {
                if let Some(trace) = core.last_cpu_trace() {
                    if recent_traces.len() == 24 {
                        let _ = recent_traces.pop_front();
                    }
                    recent_traces.push_back(trace.to_owned());
                }
            }
            Err(err) => {
                let joined = recent_traces.make_contiguous().join("\n");
                panic!(
                    "blargg cpu ROM failed at step {step}, pc={:04X}: {err}\nrecent trace:\n{joined}",
                    core.cpu_pc()
                );
            }
        }

        let status = core.read_memory(BLARGG_STATUS_ADDR);
        if status == 0x80 {
            saw_running_state = true;
            continue;
        }
        if status == 0x00 {
            if saw_running_state || step >= BLARGG_MIN_COMPLETION_STEP {
                completed = true;
                break;
            }
            continue;
        }

        let message = read_blargg_message(&core);
        panic!("blargg cpu ROM reported failure status {status:#04X} after step {step}: {message}");
    }

    assert!(
        completed,
        "blargg cpu ROM did not complete in instruction window (max steps {BLARGG_MAX_STEPS}); status={:#04X}; last message: {}",
        core.read_memory(BLARGG_STATUS_ADDR),
        read_blargg_message(&core)
    );
}
