//! Reusable testing utilities and integration harnesses for the NES emulator.
//!
//! When developing an emulator, unit tests only verify the smallest components.
//! True confidence comes from integration testing—feeding a known ROM into the `NesCore`,
//! executing it for several thousand cycles, and verifying the resulting hardware state.
//!
//! This crate prevents duplication across the workspace by providing tools to:
//! - Build and inject minimal homebrew ROMs.
//! - Trace APU register writes to ensure cycle-accurate sound events.
//! - Capture and analyze `.pcm` audio streams against "golden" known-good recordings.
//!
//! ## Examples
//!
//! Using the harness to collect audio states:
//!
//! ```rust
//! use nes_test_harness::{AudioStats, audio_stats};
//!
//! let sine_wave = [0, 16000, 32000, 16000, 0, -16000, -32000, -16000];
//! let stats = audio_stats(&sine_wave);
//! assert_eq!(stats.sample_count, 8);
//! assert_eq!(stats.peak, 32000);
//! ```

mod homebrew;
/// Provides hardcoded path resolution for testing ROMs.
pub mod rom_paths;

pub use homebrew::{build_homebrew_rom, default_homebrew_rom_path, write_homebrew_rom};
pub use rom_paths::*;

pub mod audio;
pub use audio::*;

use nes_core::{Command, CoreError, NesCore, cpu::CpuBusAccessKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApuWriteEvent {
    pub cpu_cycle: u64,
    pub addr: u16,
    pub value: u8,
}

const INES_HEADER_LEN: usize = 16;
const INES_MAGIC: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];

/// Steps the core and records all writes made to the APU registers (`0x4000`..=`0x4017`).
///
/// This is heavily used in `bbbradsmith_golden_capture` style tests. By inspecting the sequence
/// and exact cycle timestamps of these writes, we can prove the CPU execution timing is flawless.
///
/// ## Examples
///
/// ```rust
/// use nes_core::{NesCore, Command};
/// use nes_test_harness::collect_apu_register_writes;
///
/// let mut core = NesCore::new();
/// // In a real test, load a test ROM here.
/// let writes = collect_apu_register_writes(&mut core, 10).unwrap();
/// ```
pub fn collect_apu_register_writes(
    core: &mut NesCore,
    cpu_steps: u32,
) -> Result<Vec<ApuWriteEvent>, CoreError> {
    let mut writes = Vec::with_capacity(cpu_steps as usize / 16);
    for _ in 0..cpu_steps {
        core.execute(Command::StepCpu)?;
        let cpu_cycle = core.total_cycles();
        for access in core.last_cpu_bus_trace() {
            if access.kind == CpuBusAccessKind::Write && (0x4000..=0x4017).contains(&access.addr) {
                writes.push(ApuWriteEvent {
                    cpu_cycle,
                    addr: access.addr,
                    value: access.value,
                });
            }
        }
    }
    Ok(writes)
}

#[must_use]
pub fn apu_write_hash(writes: &[ApuWriteEvent]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for event in writes {
        hash ^= event.cpu_cycle;
        hash = hash.wrapping_mul(0x0000_0001_0000_01b3);
        hash ^= u64::from(event.addr);
        hash = hash.wrapping_mul(0x0000_0001_0000_01b3);
        hash ^= u64::from(event.value);
        hash = hash.wrapping_mul(0x0000_0001_0000_01b3);
    }
    hash
}

#[must_use]
pub fn detect_mapper_id(rom_bytes: &[u8]) -> Option<u16> {
    if rom_bytes.len() < INES_HEADER_LEN || rom_bytes[0..4] != INES_MAGIC {
        return None;
    }

    let flags6 = rom_bytes[6];
    let flags7 = rom_bytes[7];
    let is_nes2 = (flags7 & 0b0000_1100) == 0b0000_1000;
    let mapper_low = u16::from((flags6 >> 4) | (flags7 & 0xF0));
    if is_nes2 {
        Some(mapper_low | (u16::from(rom_bytes[8] & 0x0F) << 8))
    } else {
        Some(mapper_low)
    }
}

#[must_use]
pub fn mapper_supported_by_core(mapper_id: u16) -> bool {
    matches!(mapper_id, 0 | 1 | 2 | 4)
}

#[cfg(test)]
mod tests {
    use super::{
        apu_write_hash, collect_apu_register_writes, detect_mapper_id, mapper_supported_by_core,
    };
    use nes_core::NesCore;

    #[test]
    fn collect_apu_register_writes_tracks_apu_bus_stores() {
        let mut core = NesCore::new();
        core.load_cpu_bytes(
            0xC000,
            &[
                0xA9, 0x0F, // LDA #$0F
                0x8D, 0x00, 0x40, // STA $4000
                0x4C, 0x00, 0xC0, // JMP $C000
            ],
        );

        let writes =
            collect_apu_register_writes(&mut core, 8).expect("step cpu should not fail in loop");
        assert!(
            writes
                .iter()
                .any(|event| event.addr == 0x4000 && event.value == 0x0F),
            "expected write to $4000 in captured APU write trace"
        );

        let hash = apu_write_hash(&writes);
        assert_ne!(hash, 0);
        assert_ne!(hash, 1);

        let mut writes = writes;

        writes[0].cpu_cycle ^= 0xFFFF;
        assert_ne!(hash, apu_write_hash(&writes));
        writes[0].cpu_cycle ^= 0xFFFF;

        writes[0].addr ^= 0xFFFF;
        assert_ne!(hash, apu_write_hash(&writes));
        writes[0].addr ^= 0xFFFF;

        writes[0].value ^= 0xFF;
        assert_ne!(hash, apu_write_hash(&writes));
        writes[0].value ^= 0xFF;

        let orig_cycle = writes[0].cpu_cycle;
        writes[0].cpu_cycle |= 0xFFFF;
        if writes[0].cpu_cycle != orig_cycle {
            assert_ne!(hash, apu_write_hash(&writes));
        }
        writes[0].cpu_cycle = orig_cycle;

        let orig_addr = writes[0].addr;
        writes[0].addr &= 0x0000;
        if writes[0].addr != orig_addr {
            assert_ne!(hash, apu_write_hash(&writes));
        }
        writes[0].addr = orig_addr;

        let orig_value_or = writes[0].value;
        writes[0].value |= 0xFF;
        if writes[0].value != orig_value_or {
            assert_ne!(hash, apu_write_hash(&writes));
        }
        writes[0].value = orig_value_or;

        let orig_value_and = writes[0].value;
        writes[0].value &= 0x00;
        if writes[0].value != orig_value_and {
            assert_ne!(hash, apu_write_hash(&writes));
        }
        writes[0].value = orig_value_and;
    }

    #[test]
    fn collect_apu_register_writes_ignores_reads() {
        let mut core = NesCore::new();
        core.load_cpu_bytes(
            0xC000,
            &[
                0xAD, 0x00, 0x40, // LDA $4000
                0x4C, 0x00, 0xC0, // JMP $C000
            ],
        );

        let writes =
            collect_apu_register_writes(&mut core, 8).expect("step cpu should not fail in loop");
        assert!(writes.is_empty(), "expected reads to be ignored");
    }

    #[test]
    fn collect_apu_register_writes_ignores_non_apu_writes() {
        let mut core = NesCore::new();
        core.load_cpu_bytes(
            0xC000,
            &[
                0xA9, 0x0F, // LDA #$0F
                0x8D, 0x18, 0x40, // STA $4018 (outside APU range)
                0x4C, 0x00, 0xC0, // JMP $C000
            ],
        );

        let writes =
            collect_apu_register_writes(&mut core, 8).expect("step cpu should not fail in loop");
        assert!(
            writes.is_empty(),
            "expected out of bounds writes to be ignored"
        );
    }

    #[test]
    fn detect_mapper_id_reads_ines_header() {
        let mut rom = [0_u8; 16];
        rom[0..4].copy_from_slice(b"NES\x1A");
        rom[6] = 0x10;
        rom[7] = 0x20;
        assert_eq!(detect_mapper_id(&rom), Some(0x21));
    }

    #[test]
    fn mapper_supported_by_core_matches_core_surface() {
        assert!(mapper_supported_by_core(0));
        assert!(mapper_supported_by_core(1));
        assert!(mapper_supported_by_core(2));
        assert!(mapper_supported_by_core(4));
        assert!(!mapper_supported_by_core(69));
    }
}
