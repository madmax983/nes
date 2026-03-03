//! CPU status register (P) abstraction.
//!
//! The 6502 CPU has a processor status register that holds condition flags
//! updated by various operations. This module provides a safe interface
//! for manipulating these flags without manual bitwise logic.

/// Represents the CPU status flags.
///
/// Handles setting, clearing, and testing the various bits used by the 6502
/// (Carry, Zero, Interrupt Disable, Decimal, Overflow, Negative). It also
/// correctly manages the somewhat complicated "Break" and "Unused" flag
/// behavior when pushing/pulling to/from the stack.
///
/// ## Panics
///
/// The methods on this struct do not panic.
///
/// ## Examples
///
/// ```
/// use nes_core::cpu::status::Status;
///
/// let mut status = Status::default();
/// assert!(!status.carry());
///
/// status.set_carry(true);
/// assert!(status.carry());
///
/// status.update_zn(0x00); // Setting value to 0 updates Zero and Negative flags
/// assert!(status.zero());
/// assert!(!status.negative());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Status {
    bits: u8,
}

impl Status {
    const CARRY_BIT: u8 = 0b0000_0001;
    const ZERO_BIT: u8 = 0b0000_0010;
    const INTERRUPT_DISABLE_BIT: u8 = 0b0000_0100;
    const DECIMAL_BIT: u8 = 0b0000_1000;
    const BREAK_BIT: u8 = 0b0001_0000;
    const UNUSED_BIT: u8 = 0b0010_0000;
    const OVERFLOW_BIT: u8 = 0b0100_0000;
    const NEGATIVE_BIT: u8 = 0b1000_0000;

    /// Constructs status from raw bits.
    #[must_use]
    pub const fn with_bits(bits: u8) -> Self {
        Self { bits }
    }

    /// Returns raw status bits.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.bits
    }

    /// Carry flag (`C`).
    #[must_use]
    pub fn carry(&self) -> bool {
        self.bits & Self::CARRY_BIT != 0
    }

    /// Zero flag (`Z`).
    #[must_use]
    pub fn zero(&self) -> bool {
        self.bits & Self::ZERO_BIT != 0
    }

    /// Interrupt disable flag (`I`).
    #[must_use]
    pub fn interrupt_disable(&self) -> bool {
        self.bits & Self::INTERRUPT_DISABLE_BIT != 0
    }

    /// Overflow flag (`V`).
    #[must_use]
    pub fn overflow(&self) -> bool {
        self.bits & Self::OVERFLOW_BIT != 0
    }

    /// Negative flag (`N`).
    #[must_use]
    pub fn negative(&self) -> bool {
        self.bits & Self::NEGATIVE_BIT != 0
    }

    /// Sets or clears carry flag (`C`).
    pub fn set_carry(&mut self, enabled: bool) {
        self.set_flag(Self::CARRY_BIT, enabled);
    }

    /// Sets or clears interrupt-disable flag (`I`).
    pub fn set_interrupt_disable(&mut self, enabled: bool) {
        self.set_flag(Self::INTERRUPT_DISABLE_BIT, enabled);
    }

    /// Sets or clears decimal flag (`D`).
    pub fn set_decimal(&mut self, enabled: bool) {
        self.set_flag(Self::DECIMAL_BIT, enabled);
    }

    /// Sets or clears break flag (`B`).
    pub fn set_break(&mut self, enabled: bool) {
        self.set_flag(Self::BREAK_BIT, enabled);
    }

    /// Sets or clears overflow flag (`V`).
    pub fn set_overflow(&mut self, enabled: bool) {
        self.set_flag(Self::OVERFLOW_BIT, enabled);
    }

    /// Sets or clears negative flag (`N`).
    pub fn set_negative(&mut self, enabled: bool) {
        self.set_flag(Self::NEGATIVE_BIT, enabled);
    }

    /// Updates zero and negative flags from a result value.
    pub fn update_zn(&mut self, value: u8) {
        self.set_flag(Self::ZERO_BIT, value == 0);
        self.set_flag(Self::NEGATIVE_BIT, value & Self::NEGATIVE_BIT != 0);
    }

    /// Updates compare flags as if `lhs - rhs` were executed.
    pub fn update_compare(&mut self, lhs: u8, rhs: u8) {
        let result = lhs.wrapping_sub(rhs);
        self.set_carry(lhs >= rhs);
        self.update_zn(result);
    }

    /// Updates flags for `BIT` instruction semantics.
    pub fn update_bit_test(&mut self, lhs: u8, rhs: u8) {
        self.set_flag(Self::ZERO_BIT, lhs & rhs == 0);
        self.set_overflow(rhs & Self::OVERFLOW_BIT != 0);
        self.set_negative(rhs & Self::NEGATIVE_BIT != 0);
    }

    /// Restores status from stack pull (`PLP`/interrupt return semantics).
    pub fn restore_from_stack(&mut self, bits: u8) {
        self.bits = (bits | Self::UNUSED_BIT) & !Self::BREAK_BIT;
    }

    /// Encodes status bits for interrupt/BRK stack push.
    #[must_use]
    pub fn bits_for_stack_push(self) -> u8 {
        (self.bits | Self::UNUSED_BIT) & !Self::BREAK_BIT
    }

    /// Encodes status bits for `PHP` stack push.
    #[must_use]
    pub fn bits_for_php(self) -> u8 {
        self.bits | Self::UNUSED_BIT | Self::BREAK_BIT
    }

    fn set_flag(&mut self, mask: u8, enabled: bool) {
        if enabled {
            self.bits |= mask;
        } else {
            self.bits &= !mask;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_status_is_empty() {
        let status = Status::default();
        assert_eq!(status.bits(), 0);
        assert!(!status.carry());
        assert!(!status.zero());
        assert!(!status.interrupt_disable());
        assert!(!status.overflow());
        assert!(!status.negative());
    }

    #[test]
    fn construct_with_bits_retains_value() {
        let status = Status::with_bits(0b1100_0101);
        assert_eq!(status.bits(), 0b1100_0101);
        assert!(status.carry());
        assert!(status.interrupt_disable());
        assert!(status.overflow());
        assert!(status.negative());
        assert!(!status.zero());
    }

    #[test]
    fn individual_flags_can_be_set_and_cleared() {
        let mut status = Status::default();

        status.set_carry(true);
        assert!(status.carry());
        assert_eq!(status.bits(), Status::CARRY_BIT);
        status.set_carry(false);
        assert!(!status.carry());

        status.set_interrupt_disable(true);
        assert!(status.interrupt_disable());
        assert_eq!(status.bits(), Status::INTERRUPT_DISABLE_BIT);
        status.set_interrupt_disable(false);
        assert!(!status.interrupt_disable());

        status.set_decimal(true);
        assert_eq!(status.bits(), Status::DECIMAL_BIT);
        status.set_decimal(false);
        assert_eq!(status.bits(), 0);

        status.set_break(true);
        assert_eq!(status.bits(), Status::BREAK_BIT);
        status.set_break(false);
        assert_eq!(status.bits(), 0);

        status.set_overflow(true);
        assert!(status.overflow());
        assert_eq!(status.bits(), Status::OVERFLOW_BIT);
        status.set_overflow(false);
        assert!(!status.overflow());

        status.set_negative(true);
        assert!(status.negative());
        assert_eq!(status.bits(), Status::NEGATIVE_BIT);
        status.set_negative(false);
        assert!(!status.negative());
    }

    #[test]
    fn update_zn_sets_flags_based_on_value() {
        let mut status = Status::default();

        status.update_zn(0x00);
        assert!(status.zero());
        assert!(!status.negative());

        status.update_zn(0x7F);
        assert!(!status.zero());
        assert!(!status.negative());

        status.update_zn(0x80);
        assert!(!status.zero());
        assert!(status.negative());

        status.update_zn(0xFF);
        assert!(!status.zero());
        assert!(status.negative());
    }

    #[test]
    fn update_compare_sets_flags_correctly() {
        let cases = [
            // lhs, rhs, carry, zero, negative
            (0x05, 0x05, true, true, false),  // Equal
            (0x05, 0x03, true, false, false), // Greater than
            (0x03, 0x05, false, false, true), // Less than (negative result 0xFE)
            (0x80, 0x01, true, false, false), // Greater than (positive result 0x7F)
            (0x01, 0x80, false, false, true), // Less than (negative result 0x81)
            (0x7F, 0x80, false, false, true), // Less than (negative result 0xFF)
            (0x80, 0x7F, true, false, false), // Greater than (positive result 0x01)
        ];

        for (lhs, rhs, expected_c, expected_z, expected_n) in cases {
            let mut status = Status::default();
            status.update_compare(lhs, rhs);
            assert_eq!(
                status.carry(),
                expected_c,
                "carry failed for {} >= {}",
                lhs,
                rhs
            );
            assert_eq!(
                status.zero(),
                expected_z,
                "zero failed for {} - {}",
                lhs,
                rhs
            );
            assert_eq!(
                status.negative(),
                expected_n,
                "negative failed for {} - {}",
                lhs,
                rhs
            );
        }
    }

    #[test]
    fn update_bit_test_sets_flags_correctly() {
        let mut status = Status::default();

        // BIT uses mask, and sets N/V from memory value
        let lhs = 0b0000_1111;
        let rhs = 0b1100_0000;

        status.update_bit_test(lhs, rhs);
        assert!(status.zero()); // 0x0F & 0xC0 == 0
        assert!(status.overflow()); // bit 6 of rhs is 1
        assert!(status.negative()); // bit 7 of rhs is 1

        let lhs = 0b1111_1111;
        let rhs = 0b0011_1111;

        status.update_bit_test(lhs, rhs);
        assert!(!status.zero()); // 0xFF & 0x3F != 0
        assert!(!status.overflow()); // bit 6 of rhs is 0
        assert!(!status.negative()); // bit 7 of rhs is 0
    }

    #[test]
    fn stack_push_pull_semantics() {
        let mut status = Status::default();
        status.set_carry(true);
        status.set_negative(true);

        // Push via BRK/IRQ pushes with B clear, U set
        let irq_push = status.bits_for_stack_push();
        assert_eq!(irq_push & Status::BREAK_BIT, 0);
        assert_ne!(irq_push & Status::UNUSED_BIT, 0);

        // Push via PHP pushes with B set, U set
        let php_push = status.bits_for_php();
        assert_ne!(php_push & Status::BREAK_BIT, 0);
        assert_ne!(php_push & Status::UNUSED_BIT, 0);

        // Pull via PLP/RTI restores flags, ignores B and U
        let mut restored = Status::default();
        // Simulate pulling a byte with all bits set
        restored.restore_from_stack(0xFF);
        assert!(restored.carry());
        assert!(restored.negative());
        assert!(restored.overflow());
        assert_eq!(restored.bits() & Status::BREAK_BIT, 0); // B flag should be cleared on restore
        assert_ne!(restored.bits() & Status::UNUSED_BIT, 0); // U flag is always forced set
    }
}
