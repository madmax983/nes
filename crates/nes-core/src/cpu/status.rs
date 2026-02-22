#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Status {
    bits: u8,
}

impl Status {
    const CARRY_BIT: u8 = 0b0000_0001;
    const ZERO_BIT: u8 = 0b0000_0010;
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
    pub fn negative(&self) -> bool {
        self.bits & Self::NEGATIVE_BIT != 0
    }

    pub fn set_carry(&mut self, enabled: bool) {
        self.set_flag(Self::CARRY_BIT, enabled);
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

    fn set_flag(&mut self, mask: u8, enabled: bool) {
        if enabled {
            self.bits |= mask;
        } else {
            self.bits &= !mask;
        }
    }
}
