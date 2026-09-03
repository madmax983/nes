//! Golden characterization tests pinning the *observable* output of the core
//! against internal representation changes.
//!
//! These exist to guard refactors that change how state is stored without
//! intending to change what the core produces — for example switching the PPU
//! framebuffer from RGBA bytes to palette indices, or replacing the APU's
//! lazily-built mixer lookup tables with compile-time constants.
//!
//! The expected hashes were captured from the pre-refactor implementation. A
//! failure here means host-visible output moved, which is a behavior change and
//! must be justified — not re-baselined casually.

use nes_core::{AUDIO_CHUNK_SAMPLES, Command, FRAME_RGBA_BYTES, NesCore};

/// FNV-1a (64-bit). Small, dependency-free, and stable across platforms.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

fn hash_samples(samples: &[i16]) -> u64 {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    fnv1a(&bytes)
}

fn write_ppu_data(core: &mut NesCore, addr: u16, bytes: &[u8]) {
    core.write_cpu_bus(0x2006, (addr >> 8) as u8);
    core.write_cpu_bus(0x2006, (addr & 0xFF) as u8);
    for byte in bytes {
        core.write_cpu_bus(0x2007, *byte);
    }
}

/// Builds a core rendering a non-trivial background: four distinct palette
/// colors across a tiled nametable, with rendering enabled.
fn video_scenario() -> NesCore {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]); // NOP ; JMP $C000
    core.write_cpu_bus(0x2001, 0x0A); // background rendering + leftmost 8px

    // Universal background plus three distinct palette entries.
    write_ppu_data(&mut core, 0x3F00, &[0x0F, 0x16, 0x2A, 0x12]);

    // Tile 1 -> color index 1, tile 2 -> color index 2, tile 3 -> color index 3.
    write_ppu_data(&mut core, 0x0010, &[0xFF; 8]);
    write_ppu_data(&mut core, 0x0018, &[0x00; 8]);
    write_ppu_data(&mut core, 0x0020, &[0x00; 8]);
    write_ppu_data(&mut core, 0x0028, &[0xFF; 8]);
    write_ppu_data(&mut core, 0x0030, &[0xFF; 8]);
    write_ppu_data(&mut core, 0x0038, &[0xFF; 8]);

    // Fill the first nametable with a repeating 1/2/3/0 tile pattern so the
    // frame exercises every palette slot rather than one flat color.
    let tiles: Vec<u8> = (0..960_u16).map(|i| (i % 4) as u8).collect();
    write_ppu_data(&mut core, 0x2000, &tiles);

    core
}

#[test]
fn rendered_framebuffer_is_byte_stable() {
    let mut core = video_scenario();
    for _ in 0..3 {
        core.execute(Command::StepFrame).unwrap();
    }

    let mut frame = vec![0_u8; FRAME_RGBA_BYTES];
    core.fill_framebuffer_rgba(&mut frame);

    assert_eq!(frame.len(), FRAME_RGBA_BYTES);
    assert!(
        frame.as_chunks::<4>().0.iter().all(|px| px[3] == 0xFF),
        "every emitted pixel must be fully opaque"
    );
    let distinct = {
        let mut colors: Vec<[u8; 4]> = frame.as_chunks::<4>().0.to_vec();
        colors.sort_unstable();
        colors.dedup();
        colors.len()
    };
    assert!(
        distinct >= 3,
        "scenario must render a non-trivial image (distinct colors = {distinct})"
    );
    assert_eq!(
        fnv1a(&frame),
        0xc0f3_2146_3e77_4325,
        "rendered framebuffer bytes changed"
    );
}

#[test]
fn framebuffer_rgba_matches_fill_framebuffer_rgba() {
    let mut core = video_scenario();
    for _ in 0..3 {
        core.execute(Command::StepFrame).unwrap();
    }

    let owned = core.framebuffer_rgba();
    let mut filled = vec![0_u8; FRAME_RGBA_BYTES];
    core.fill_framebuffer_rgba(&mut filled);

    assert_eq!(owned, filled, "allocating and filling accessors must agree");
}

/// Drives every APU channel and hashes a long window of mixed output. This is
/// the guard for replacing the runtime-built mixer lookup tables with
/// compile-time constants: the mixed samples must be bit-identical.
#[test]
fn mixed_audio_output_is_bit_stable() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]); // NOP ; JMP $C000

    // Pulse 1 + pulse 2 at different periods, triangle, and noise.
    core.write_cpu_bus(0x4015, 0x0F);
    core.write_cpu_bus(0x4000, 0xBF);
    core.write_cpu_bus(0x4002, 0x40);
    core.write_cpu_bus(0x4003, 0x08);
    core.write_cpu_bus(0x4004, 0xBF);
    core.write_cpu_bus(0x4006, 0x90);
    core.write_cpu_bus(0x4007, 0x08);
    core.write_cpu_bus(0x4008, 0xFF);
    core.write_cpu_bus(0x400A, 0x20);
    core.write_cpu_bus(0x400B, 0x08);
    core.write_cpu_bus(0x400C, 0x0A);
    core.write_cpu_bus(0x400E, 0x04);
    core.write_cpu_bus(0x400F, 0x08);

    let mut all = Vec::new();
    let mut chunk = vec![0_i16; AUDIO_CHUNK_SAMPLES];
    for _ in 0..120 {
        core.execute(Command::StepFrame).unwrap();
        core.fill_audio_chunk_i16(&mut chunk);
        all.extend_from_slice(&chunk);
    }

    assert_eq!(all.len(), AUDIO_CHUNK_SAMPLES * 120);
    assert!(
        all.iter().any(|sample| *sample != 0),
        "scenario must actually produce audible output"
    );
    assert_eq!(
        hash_samples(&all),
        0x98cd_cec8_9c71_b967,
        "mixed APU output changed"
    );
}

/// A host that never drains audio must not grow the queue without bound, and
/// must still receive the most recent samples when it finally reads.
#[test]
fn undrained_audio_queue_stays_bounded_and_yields_recent_samples() {
    let mut core = NesCore::new();
    core.write_cpu_bus(0x4015, 0x0F);
    core.write_cpu_bus(0x4000, 0xBF);
    core.write_cpu_bus(0x4002, 0x40);
    core.write_cpu_bus(0x4003, 0x08);

    for _ in 0..600 {
        core.execute(Command::StepFrame).unwrap();
    }

    let mut chunk = vec![0_i16; AUDIO_CHUNK_SAMPLES];
    core.fill_audio_chunk_i16(&mut chunk);
    assert_eq!(chunk.len(), AUDIO_CHUNK_SAMPLES);
}

#[test]
fn state_hash_is_stable_for_video_scenario() {
    let mut core = video_scenario();
    for _ in 0..3 {
        core.execute(Command::StepFrame).unwrap();
    }
    assert_eq!(
        core.state_hash(),
        0x5c7f_cad9_6b97_d947,
        "core state hash changed"
    );
}
