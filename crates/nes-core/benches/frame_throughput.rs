/// Baseline throughput benchmarks for nes-core hot paths.
///
/// Covers three granularities of emulation stepping:
///   - `step_frame`     : ~89,341 CPU cycles (one full PPU frame)
///   - `step_scanline`  : 341 PPU dots (one scanline)
///   - `step_cpu`       : one 6502 instruction (2-8 cycles)
///
/// Run with:
///   cargo bench -p nes-core
///
/// Compare before/after an optimization with:
///   cargo bench -p nes-core -- --save-baseline before
///   # (make changes)
///   cargo bench -p nes-core -- --baseline before
use nes_core::{Command, NesCore};

fn main() {
    divan::main();
}

/// NOP ; JMP $C000 — infinite NOP loop.
/// Exercises the full emulation pipeline (CPU decode, PPU dots, APU ticks)
/// without any ROM data dependency.
const NOP_LOOP: &[u8] = &[0xEA, 0x4C, 0x00, 0xC0];

fn make_nop_core() -> NesCore {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, NOP_LOOP);
    core
}

fn make_nop_core_no_trace() -> NesCore {
    let mut core = make_nop_core();
    core.set_trace_enabled(false);
    core
}

/// Single-frame throughput — how fast can we emulate one PPU frame?
///
/// Cold: fresh NesCore each sample (first-frame cost).
/// Warm: same core accumulating frames (steady-state throughput).
#[divan::bench]
fn step_frame_cold(bencher: divan::Bencher) {
    bencher
        .with_inputs(make_nop_core)
        .bench_values(|mut core| {
            core.execute(Command::StepFrame).unwrap();
            core
        });
}

#[divan::bench]
fn step_frame_warm(bencher: divan::Bencher) {
    let mut core = make_nop_core();
    bencher.bench_local(|| {
        core.execute(Command::StepFrame).unwrap();
    });
}

/// Scanline throughput — one 341-dot scanline.
///
/// Isolates per-scanline work (sprite evaluation, overflow check)
/// from per-frame and per-instruction overhead.
#[divan::bench]
fn step_scanline_warm(bencher: divan::Bencher) {
    let mut core = make_nop_core();
    bencher.bench_local(|| {
        core.execute(Command::StepScanline).unwrap();
    });
}

/// CPU instruction throughput — one 6502 NOP (2 CPU cycles, 6 PPU dots).
///
/// Measures raw cost of `advance_hardware_cycles(2)` plus dispatch overhead.
#[divan::bench]
fn step_cpu_nop(bencher: divan::Bencher) {
    let mut core = make_nop_core();
    bencher.bench_local(|| {
        core.execute(Command::StepCpu).unwrap();
    });
}

/// Sixty-frame burst — equivalent to 1 second at 60 Hz.
///
/// Exposes steady-state throughput and amortized frame-boundary costs
/// (odd-frame skip, VBlank NMI dispatch).
#[divan::bench]
fn sixty_frames_burst(bencher: divan::Bencher) {
    let mut core = make_nop_core();
    bencher.bench_local(|| {
        for _ in 0..60 {
            core.execute(Command::StepFrame).unwrap();
        }
    });
}

/// Same benchmarks with `trace_enabled = false` — eliminates bus-trace RefCell
/// borrow overhead and format_trace String allocations from the hot path.
mod no_trace {
    use super::*;

    #[divan::bench]
    fn step_frame_warm(bencher: divan::Bencher) {
        let mut core = make_nop_core_no_trace();
        bencher.bench_local(|| {
            core.execute(Command::StepFrame).unwrap();
        });
    }

    #[divan::bench]
    fn step_cpu_nop(bencher: divan::Bencher) {
        let mut core = make_nop_core_no_trace();
        bencher.bench_local(|| {
            core.execute(Command::StepCpu).unwrap();
        });
    }

    #[divan::bench]
    fn sixty_frames_burst(bencher: divan::Bencher) {
        let mut core = make_nop_core_no_trace();
        bencher.bench_local(|| {
            for _ in 0..60 {
                core.execute(Command::StepFrame).unwrap();
            }
        });
    }
}
