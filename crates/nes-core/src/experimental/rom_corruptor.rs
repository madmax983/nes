//! Experimental tools for intentionally corrupting NES ROM data.
//!
//! This module provides the [`RomCorruptor`], an experimental utility that applies
//! deterministic chaos to PRG or CHR ROM data. It uses a lightweight Linear Congruential
//! Generator (LCG) to ensure reproducible corruptions without pulling in external `rand`
//! dependencies.

#[cfg(feature = "nova")]
/// A deterministic glitch generator for NES ROM data.
///
/// `RomCorruptor` mutates ROM byte slices by either flipping random bits or
/// scrambling random bytes. This is useful for chaos engineering, speedrun
/// "glitch modes", or simply generating cursed visual aesthetics.
pub struct RomCorruptor {
    state: u32,
}

#[cfg(feature = "nova")]
impl RomCorruptor {
    /// Creates a new `RomCorruptor` seeded with a specific value.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::rom_corruptor::RomCorruptor;
    /// let mut corruptor = RomCorruptor::new(1337);
    /// ```
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self {
            // Prevent a 0 seed from locking the state if we used a multiplicative generator,
            // though our LCG constants prevent this anyway.
            state: if seed == 0 { 0xDEAD_BEEF } else { seed },
        }
    }

    /// Advances the internal LCG state and returns a pseudo-random `u32`.
    fn next_u32(&mut self) -> u32 {
        // Simple LCG (Numerical Recipes constants)
        self.state = self.state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.state
    }

    /// Returns a pseudo-random number in `[0, range)`.
    fn next_range(&mut self, range: u32) -> u32 {
        if range == 0 {
            return 0;
        }
        self.next_u32() % range
    }

    /// Randomly corrupts the given PRG ROM data by randomizing bytes.
    ///
    /// `intensity_permille` determines the likelihood of any given byte being corrupted.
    /// For example, an intensity of `10` means roughly 1% of the data will be scrambled.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::rom_corruptor::RomCorruptor;
    /// let mut corruptor = RomCorruptor::new(42);
    /// let mut prg = vec![0xEA; 100]; // 100 NOPs
    /// corruptor.corrupt_prg(&mut prg, 50); // 5% corruption
    /// assert!(prg.iter().any(|&b| b != 0xEA));
    /// ```
    pub fn corrupt_prg(&mut self, prg: &mut [u8], intensity_permille: u16) {
        if intensity_permille == 0 || prg.is_empty() {
            return;
        }

        let intensity = u32::from(intensity_permille).min(1000);
        for byte in prg.iter_mut() {
            if self.next_range(1000) < intensity {
                // Completely scramble the instruction/data
                *byte = (self.next_u32() & 0xFF) as u8;
            }
        }
    }

    /// Randomly corrupts the given CHR ROM data by flipping bits.
    ///
    /// Flipping bits in CHR RAM tends to create interesting "static" or
    /// scrambled sprite visual artifacts rather than crashing the emulator.
    ///
    /// `intensity_permille` determines the likelihood of a single byte having
    /// a random bit flipped.
    pub fn corrupt_chr(&mut self, chr: &mut [u8], intensity_permille: u16) {
        if intensity_permille == 0 || chr.is_empty() {
            return;
        }

        let intensity = u32::from(intensity_permille).min(1000);
        for byte in chr.iter_mut() {
            if self.next_range(1000) < intensity {
                // Flip a random bit (0-7)
                let bit = self.next_range(8) as u8;
                *byte ^= 1 << bit;
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn corruptor_modifies_prg_deterministically() {
        let mut prg1 = vec![0x00; 1024];
        let mut prg2 = vec![0x00; 1024];

        let mut corruptor1 = RomCorruptor::new(12345);
        corruptor1.corrupt_prg(&mut prg1, 100);

        let mut corruptor2 = RomCorruptor::new(12345);
        corruptor2.corrupt_prg(&mut prg2, 100);

        // They should be identical due to same seed
        assert_eq!(prg1, prg2);

        // They should be modified from the original
        let modified_count = prg1.iter().filter(|&&b| b != 0x00).count();
        assert!(modified_count > 0);
        // Should be roughly 10% (100 permille)
        assert!((50..150).contains(&modified_count));
    }

    #[test]
    fn corruptor_modifies_chr_with_bit_flips() {
        let mut chr = vec![0xFF; 256];
        let mut corruptor = RomCorruptor::new(999);

        // 50% corruption
        corruptor.corrupt_chr(&mut chr, 500);

        let mut flipped_bits = 0;
        for &byte in &chr {
            if byte != 0xFF {
                // It should only have 1 bit flipped per affected byte
                assert_eq!(byte.count_ones(), 7);
                flipped_bits += 1;
            }
        }

        assert!(flipped_bits > 0);
    }

    #[test]
    fn zero_intensity_does_nothing() {
        let mut data = vec![0xAA; 100];
        let mut corruptor = RomCorruptor::new(1);
        corruptor.corrupt_prg(&mut data, 0);
        assert!(data.iter().all(|&b| b == 0xAA));

        corruptor.corrupt_chr(&mut data, 0);
        assert!(data.iter().all(|&b| b == 0xAA));
    }

    #[test]
    fn lcg_avoids_zero_state_lock() {
        let mut corruptor = RomCorruptor::new(0);
        assert_ne!(corruptor.next_u32(), 0);
    }
}
