//! Guards the core's live memory footprint.
//!
//! `nes-core` is intended to run on memory-constrained hosts (a bare-metal
//! ESP32-class target has on the order of 300-500KB of usable RAM in total),
//! so the working set of a running core is a budgeted resource, not an
//! incidental detail. This test measures it directly with a counting allocator
//! and fails if it regresses.
//!
//! Accounting is per-thread. A process-global counter would attribute every
//! other concurrently running test's allocations — including the tens of
//! megabytes a panicking test spends symbolizing its backtrace — to whichever
//! measurement happened to be in flight.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use nes_core::{AUDIO_CHUNK_SAMPLES, Command, NesCore};

thread_local! {
    /// Bytes currently allocated by this thread. `const` initialization keeps
    /// the first access from allocating and re-entering the allocator.
    static LIVE_BYTES: Cell<isize> = const { Cell::new(0) };
}

fn adjust_live(delta: isize) {
    // `try_with` so an allocation during TLS teardown is ignored rather than
    // panicking inside the allocator.
    let _ = LIVE_BYTES.try_with(|live| live.set(live.get() + delta));
}

fn live_bytes() -> isize {
    LIVE_BYTES.try_with(Cell::get).unwrap_or(0)
}

struct CountingAllocator;

// SAFETY: every method forwards to `System` and only adds bookkeeping around it.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            adjust_live(layout.size() as isize);
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            adjust_live(layout.size() as isize);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        adjust_live(-(layout.size() as isize));
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            adjust_live(new_size as isize - layout.size() as isize);
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Heap held by a running core, excluding anything the test itself allocates.
fn measure_running_core_heap(frames: u32) -> usize {
    let rom = build_nrom_rom();
    let mut chunk = vec![0_i16; AUDIO_CHUNK_SAMPLES];

    // Everything the test owns is allocated before the baseline is taken.
    let before = live_bytes();
    let mut core = NesCore::new();
    core.load_ines_rom(&rom)
        .expect("synthetic NROM should load");

    for _ in 0..frames {
        core.execute(Command::StepFrame).expect("frame should step");
        core.fill_audio_chunk_i16(&mut chunk);
    }

    let held = live_bytes() - before;
    drop(core);
    usize::try_from(held).expect("core heap accounting should be non-negative")
}

/// Minimal 16KB NROM image: reset vector into an infinite loop.
fn build_nrom_rom() -> Vec<u8> {
    let mut rom = vec![0_u8; 16 + 16 * 1024];
    rom[0..4].copy_from_slice(&[0x4E, 0x45, 0x53, 0x1A]);
    rom[4] = 1; // 1 x 16KB PRG
    rom[5] = 0; // CHR RAM
    let prg = &mut rom[16..];
    prg[0x3FF0..0x3FF4].copy_from_slice(&[0xEA, 0x4C, 0xF0, 0xFF]); // NOP ; JMP $FFF0
    prg[0x3FFC] = 0xF0; // reset vector low
    prg[0x3FFD] = 0xFF; // reset vector high
    rom
}

/// The `NesCore` value itself. Dominated by the CPU's flat 64KB address-space
/// array, which is the next structural reduction on the embedded roadmap.
#[test]
fn core_struct_size_is_bounded() {
    let size = std::mem::size_of::<NesCore>();
    assert!(
        size <= 96 * 1024,
        "NesCore struct grew to {size} bytes (budget 96KB)"
    );
}

#[test]
fn running_core_heap_is_bounded() {
    // Measured at ~98KB for a 16KB NROM after the framebuffer, mixer-table and
    // audio-queue reductions (it was ~578KB before). The budget leaves room for
    // larger cartridges without silently absorbing a new large allocation.
    let heap = measure_running_core_heap(600);
    assert!(
        heap <= 160 * 1024,
        "running core heap grew to {heap} bytes (budget 160KB)"
    );
}

/// A long run must not grow the working set — no unbounded queue or cache.
#[test]
fn running_core_heap_does_not_grow_over_time() {
    let short_run = measure_running_core_heap(60);
    let long_run = measure_running_core_heap(1_200);
    assert_eq!(
        short_run, long_run,
        "heap grew between a 60-frame and a 1200-frame run"
    );
}
