use nes_core::Button;

/// Defines the discrete action space used by the model for controlling the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAction {
    /// Do nothing.
    Noop,
    /// Press only the Right D-pad button.
    Right,
    /// Press Right and A simultaneously (e.g. running jump).
    RightA,
    /// Press only the A button (e.g. standing jump).
    A,
    /// Press Right and B simultaneously (e.g. running).
    RightB,
    /// Press Right, A, and B simultaneously (e.g. fast running jump).
    RightAB,
}

impl ControlAction {
    /// Returns the total number of actions in the space.
    #[must_use]
    pub const fn action_count() -> usize {
        6
    }

    /// Maps the abstract action to the exact 8-bit NES controller bitfield.
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
