//! The Puppeteer's Strings: Discrete Control Actions
//!
//! Reinforcement learning agents cannot press buttons. They output floating-point tensors.
//! This module bridges the gap between the cold math of a neural network and the
//! tactile reality of an 8-bit game controller.
//!
//! By restricting the agent to a small, curated set of actions, we drastically
//! reduce the exploration space and guide the agent toward meaningful gameplay rather
//! than mashing `Start` and `Select` in a corner.

use nes_core::Button;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The constrained vocabulary our AI uses to speak to the NES.
///
/// Rather than exploring all 256 mathematically possible button combinations on an NES joypad,
/// this enum curates the exact dialect needed for most side-scrolling platformers.
/// The agent points at one of these discrete thoughts, and we pull the strings on the
/// controller port.
pub enum ControlAction {
    /// The agent does nothing. The digital equivalent of a blank stare.
    Noop,
    /// Walking forward. The foundation of any side-scrolling adventure.
    Right,
    /// Moving forward while leaping. Critical for avoiding pits and stomping goombas.
    RightA,
    /// A vertical jump in place. Useful for hitting blocks directly overhead.
    A,
    /// Sprinting forward. Fast, but dangerous.
    RightB,
    /// A sprinting leap forward. Maximum momentum, maximum risk.
    RightAB,
}

impl ControlAction {
    /// Returns the total number of distinct actions in the action space.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_ai::actions::ControlAction;
    ///
    /// assert_eq!(ControlAction::action_count(), 6);
    /// ```
    #[must_use]
    pub const fn action_count() -> usize {
        6
    }

    /// Converts the abstract action into the raw 8-bit bitmask expected by the NES controller port.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_ai::actions::ControlAction;
    /// use nes_core::Button;
    ///
    /// let action = ControlAction::RightA;
    /// let expected = Button::Right.bit_mask() | Button::A.bit_mask();
    /// assert_eq!(action.controller1_bits(), expected);
    /// ```
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
