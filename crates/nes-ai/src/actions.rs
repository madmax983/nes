use nes_core::Button;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAction {
    Noop,
    Right,
    RightA,
    A,
    RightB,
    RightAB,
}

impl ControlAction {
    #[must_use]
    pub const fn action_count() -> usize {
        6
    }

    #[must_use]
    pub fn controller1_bits(self) -> u8 {
        match self {
            Self::Noop => 0,
            Self::Right => Button::Right.bit_mask(),
            Self::RightA => Button::Right.bit_mask() | Button::A.bit_mask(),
            Self::A => Button::A.bit_mask(),
            Self::RightB => Button::Right.bit_mask() | Button::B.bit_mask(),
            Self::RightAB => Button::Right.bit_mask() | Button::A.bit_mask() | Button::B.bit_mask(),
        }
    }
}
