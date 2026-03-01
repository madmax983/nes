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
/// use nes_core::Status;
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
