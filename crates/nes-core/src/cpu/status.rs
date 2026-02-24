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

    #[must_use]
    pub const fn with_bits(bits: u8) -> Self {
        Self { bits }
    }

    #[must_use]
    pub const fn bits(self) -> u8 {
        self.bits
    }

    #[must_use]
    pub fn carry(&self) -> bool {
        self.bits & Self::CARRY_BIT != 0
    }

    #[must_use]
    pub fn zero(&self) -> bool {
        self.bits & Self::ZERO_BIT != 0
    }

    #[must_use]
    pub fn interrupt_disable(&self) -> bool {
        self.bits & Self::INTERRUPT_DISABLE_BIT != 0
    }

    #[must_use]
    pub fn overflow(&self) -> bool {
        self.bits & Self::OVERFLOW_BIT != 0
    }

    #[must_use]
    pub fn negative(&self) -> bool {
        self.bits & Self::NEGATIVE_BIT != 0
    }

    pub fn set_carry(&mut self, enabled: bool) {
        self.set_flag(Self::CARRY_BIT, enabled);
    }

    pub fn set_interrupt_disable(&mut self, enabled: bool) {
        self.set_flag(Self::INTERRUPT_DISABLE_BIT, enabled);
    }

    pub fn set_decimal(&mut self, enabled: bool) {
        self.set_flag(Self::DECIMAL_BIT, enabled);
    }

    pub fn set_break(&mut self, enabled: bool) {
        self.set_flag(Self::BREAK_BIT, enabled);
    }

    pub fn set_overflow(&mut self, enabled: bool) {
        self.set_flag(Self::OVERFLOW_BIT, enabled);
    }

    pub fn set_negative(&mut self, enabled: bool) {
        self.set_flag(Self::NEGATIVE_BIT, enabled);
    }

    pub fn update_zn(&mut self, value: u8) {
        self.set_flag(Self::ZERO_BIT, value == 0);
        self.set_flag(Self::NEGATIVE_BIT, value & Self::NEGATIVE_BIT != 0);
    }

    pub fn update_compare(&mut self, lhs: u8, rhs: u8) {
        let result = lhs.wrapping_sub(rhs);
        self.set_carry(lhs >= rhs);
        self.update_zn(result);
    }

    pub fn update_bit_test(&mut self, lhs: u8, rhs: u8) {
        self.set_flag(Self::ZERO_BIT, lhs & rhs == 0);
        self.set_overflow(rhs & Self::OVERFLOW_BIT != 0);
        self.set_negative(rhs & Self::NEGATIVE_BIT != 0);
    }

    #[must_use]
    pub fn bits_for_stack_push(self) -> u8 {
        (self.bits | Self::UNUSED_BIT) & !Self::BREAK_BIT
    }

    fn set_flag(&mut self, mask: u8, enabled: bool) {
        if enabled {
            self.bits |= mask;
        } else {
            self.bits &= !mask;
        }
    }
}
