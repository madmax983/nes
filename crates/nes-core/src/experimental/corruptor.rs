//! ROM Corruptor (Nova Experiment)
//!
//! A tool that injects deterministic entropy into iNES ROMs to stress test the emulator,
//! find edge-case crashes, or create "glitch art".
//!
//! It strictly mutates only the PRG and CHR payload bytes, leaving the 16-byte iNES
//! header and any potential 512-byte trainer intact to ensure the ROM remains loadable.

/// A simple, dependency-free Linear Congruential Generator for deterministic entropy.
#[derive(Debug, Clone)]
pub struct Lcg {
    state: u64,
}

impl Lcg {
    /// Creates a new LCG with the given seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Returns the next pseudo-random `u32`.
    pub fn next_u32(&mut self) -> u32 {
        // Using the MMIX LCG parameters.
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }

    /// Returns a pseudo-random value between `0` and `max - 1`.
    pub fn next_in_range(&mut self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (self.next_u32() as usize) % max
    }
}

/// The types of corruption that can be applied to the ROM data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorruptionMode {
    /// Randomly flips a single bit in a byte.
    BitRot,
    /// Replaces a byte with a completely random byte.
    Randomize,
    /// Swaps the value of two adjacent bytes.
    ByteSwap,
}

/// A tool to corrupt an iNES ROM deterministically.
#[derive(Debug, Clone)]
pub struct RomCorruptor {
    rng: Lcg,
    mode: CorruptionMode,
    intensity: usize,
}

impl RomCorruptor {
    /// Creates a new corruptor.
    ///
    /// `intensity` determines approximately how many bytes will be corrupted (e.g. 1 out of `intensity` bytes).
    /// Lower intensity means more corruption. A value of 0 means no corruption.
    #[must_use]
    pub fn new(seed: u64, mode: CorruptionMode, intensity: usize) -> Self {
        Self {
            rng: Lcg::new(seed),
            mode,
            intensity,
        }
    }

    /// Corrupts the given ROM bytes in-place.
    ///
    /// Only modifies the PRG and CHR payload. The 16-byte header and any trainer are skipped.
    /// If the ROM is smaller than the header, it does nothing.
    pub fn corrupt(&mut self, rom: &mut [u8]) {
        if rom.len() < 16 || self.intensity == 0 {
            return;
        }

        // Parse header manually to determine PRG/CHR start to avoid duplicating `nes_core::rom` logic
        // which might return a read-only view.
        let flags6 = rom[6];
        let has_trainer = (flags6 & 0b0000_0100) != 0;

        let start_offset = 16 + if has_trainer { 512 } else { 0 };

        if start_offset >= rom.len() {
            return;
        }

        let payload_len = rom.len() - start_offset;
        // Approximate number of corruptions based on intensity
        let num_corruptions = payload_len / self.intensity;

        for _ in 0..num_corruptions {
            let offset = start_offset + self.rng.next_in_range(payload_len);

            match self.mode {
                CorruptionMode::BitRot => {
                    let bit = self.rng.next_in_range(8);
                    rom[offset] ^= 1 << bit;
                }
                CorruptionMode::Randomize => {
                    rom[offset] = (self.rng.next_u32() & 0xFF) as u8;
                }
                CorruptionMode::ByteSwap => {
                    if offset + 1 < rom.len() {
                        rom.swap(offset, offset + 1);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_dummy_rom(has_trainer: bool) -> Vec<u8> {
        let mut rom = vec![0; 16 + 1024]; // 16 byte header + 1024 bytes payload
        // set some standard signature
        rom[0] = 0x4E;
        rom[1] = 0x45;
        rom[2] = 0x53;
        rom[3] = 0x1A;
        if has_trainer {
            rom[6] |= 0b0000_0100;
            rom.extend(vec![0; 512]); // Add trainer space implicitly by shifting data
        }
        rom
    }

    #[test]
    fn test_lcg_deterministic() {
        let mut lcg1 = Lcg::new(42);
        let mut lcg2 = Lcg::new(42);

        assert_eq!(lcg1.next_u32(), lcg2.next_u32());
        assert_eq!(lcg1.next_u32(), lcg2.next_u32());
        assert_eq!(lcg1.next_u32(), lcg2.next_u32());
    }

    #[test]
    fn test_corrupt_leaves_header_intact() {
        let original_rom = create_dummy_rom(false);
        let mut rom = original_rom.clone();

        let mut corruptor = RomCorruptor::new(123, CorruptionMode::Randomize, 1);
        corruptor.corrupt(&mut rom);

        assert_eq!(&original_rom[0..16], &rom[0..16]);
    }

    #[test]
    fn test_corrupt_leaves_trainer_intact() {
        let mut original_rom = create_dummy_rom(true);
        // fill trainer with 1s
        for item in original_rom.iter_mut().skip(16).take(512) {
            *item = 1;
        }

        let mut rom = original_rom.clone();
        let mut corruptor = RomCorruptor::new(123, CorruptionMode::Randomize, 1);
        corruptor.corrupt(&mut rom);

        assert_eq!(&original_rom[0..16], &rom[0..16]);
        assert_eq!(&original_rom[16..16 + 512], &rom[16..16 + 512]);
    }

    #[test]
    fn test_corrupt_deterministic() {
        let mut rom1 = create_dummy_rom(false);
        let mut rom2 = create_dummy_rom(false);

        let mut corruptor1 = RomCorruptor::new(999, CorruptionMode::BitRot, 10);
        let mut corruptor2 = RomCorruptor::new(999, CorruptionMode::BitRot, 10);

        corruptor1.corrupt(&mut rom1);
        corruptor2.corrupt(&mut rom2);

        assert_eq!(rom1, rom2);
    }
}
