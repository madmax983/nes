//! Actions that the AI agent can take in the environment.
//!
//! This module defines the discrete action space for our reinforcement learning
//! agents. Rather than allowing the AI to press any combination of the 8 NES
//! buttons (which would result in $2^8 = 256$ possible actions, many of which
//! are useless or contradictory like pressing Left and Right simultaneously),
//! we restrict the agent to a small, curated list of meaningful inputs.

use nes_core::Button;

/// The discrete set of actions available to the reinforcement learning agent.
///
/// This is a heavily pruned subset of the NES controller space, specifically
/// tailored for side-scrolling platformers where holding Right and jumping/running
/// are the primary mechanics.
///
/// # Examples
///
/// ```
/// use nes_ai::actions::ControlAction;
///
/// // The agent decides to run and jump to the right.
/// let action = ControlAction::RightAB;
///
/// // We translate this high-level intent into raw NES controller bits.
/// let bits = action.controller1_bits();
/// assert_eq!(bits, 0b1000_0011); // Right (0x80) | B (0x02) | A (0x01)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAction {
    /// Do nothing. Just let the frame pass.
    Noop,
    /// Press the D-Pad Right. Used for walking.
    Right,
    /// Press the D-Pad Right and the A button simultaneously. Used for jumping forward.
    RightA,
    /// Press the A button alone. Used for jumping straight up.
    A,
    /// Press the D-Pad Right and the B button simultaneously. Used for running.
    RightB,
    /// Press the D-Pad Right, the A button, and the B button simultaneously. Used for running jumps.
    RightAB,
}

impl ControlAction {
    /// Returns the total number of discrete actions available in this action space.
    ///
    /// This is used to size the output layer of the policy network.
    #[must_use]
    pub const fn action_count() -> usize {
        6
    }

    /// Translates the high-level semantic action into a raw 8-bit NES controller state.
    ///
    /// The resulting byte uses the standard NES controller mapping where:
    /// Bit 7: Right, Bit 6: Left, Bit 5: Down, Bit 4: Up,
    /// Bit 3: Start, Bit 2: Select, Bit 1: B, Bit 0: A
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
