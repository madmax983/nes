use nes_core::Button;

/// Represents the simplified discrete action space available to the AI agent during training.
///
/// Instead of a full 8-button permutation (256 combinations), the agent is restricted
/// to a smaller subset of actions that are useful for progressing in standard platforming games
/// (e.g. running right, jumping).
///
/// # Examples
///
/// ```
/// use nes_ai::actions::ControlAction;
///
/// let action = ControlAction::RightA;
/// let bits = action.controller1_bits();
///
/// // Right (bit 7) and A (bit 0) are set.
/// assert_eq!(bits, 129);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAction {
    /// No buttons are pressed.
    Noop,
    /// Only the D-pad Right button is pressed.
    Right,
    /// D-pad Right and A are pressed simultaneously (running jump).
    RightA,
    /// Only the A button is pressed (standing jump).
    A,
    /// D-pad Right and B are pressed simultaneously (running fast).
    RightB,
    /// D-pad Right, A, and B are pressed simultaneously (running fast jump).
    RightAB,
}

impl ControlAction {
    /// Returns the total number of discrete actions available in this action space.
    #[must_use]
    pub const fn action_count() -> usize {
        6
    }

    /// Converts the high-level `ControlAction` into the raw 8-bit bitmask expected
    /// by the NES controller hardware.
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
