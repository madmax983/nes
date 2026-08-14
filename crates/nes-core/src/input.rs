pub(crate) const CONTROLLER_OPEN_BUS_MASK: u8 = 0x40;

/// Represents a standard NES controller button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, core::hash::Hash)]
pub enum Button {
    /// A button.
    A,
    /// B button.
    B,
    /// Select button.
    Select,
    /// Start button.
    Start,
    /// Up direction.
    Up,
    /// Down direction.
    Down,
    /// Left direction.
    Left,
    /// Right direction.
    Right,
}

impl Button {
    /// Returns this button's bit in controller bitfields.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::Button;
    /// assert_eq!(Button::A.bit_mask(), 0b0000_0001);
    /// assert_eq!(Button::Start.bit_mask(), 0b0000_1000);
    /// ```
    #[must_use]
    pub fn bit_mask(self) -> u8 {
        match self {
            Self::A => 0b0000_0001,
            Self::B => 0b0000_0010,
            Self::Select => 0b0000_0100,
            Self::Start => 0b0000_1000,
            Self::Up => 0b0001_0000,
            Self::Down => 0b0010_0000,
            Self::Left => 0b0100_0000,
            Self::Right => 0b1000_0000,
        }
    }
}

/// Represents the controller port for a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    /// Controller port 1.
    One,
    /// Controller port 2.
    Two,
}

impl Player {
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::One => 0,
            Self::Two => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ControllerState {
    pub(crate) bits: u8,
    pub(crate) shift: u8,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ControllerPorts {
    pub(crate) controllers: [ControllerState; 2],
    pub(crate) controller_strobe: bool,
}

impl ControllerPorts {
    pub(crate) fn set_controller_bits(&mut self, bits: u8, player: Player) {
        let state = &mut self.controllers[player.index()];
        state.bits = bits;
        if self.controller_strobe {
            state.shift = bits;
        }
    }

    pub(crate) fn write_controller_strobe(&mut self, value: u8) {
        let next_strobe = value & 1 != 0;
        if next_strobe {
            self.controller_strobe = true;
            for c in &mut self.controllers {
                c.shift = c.bits;
            }
            return;
        }

        if self.controller_strobe {
            for c in &mut self.controllers {
                c.shift = c.bits;
            }
        }
        self.controller_strobe = false;
    }

    pub(crate) fn controller_port_sample(&self, player: Player) -> u8 {
        let state = &self.controllers[player.index()];
        let bit = if self.controller_strobe {
            state.bits & 1
        } else {
            state.shift & 1
        };
        bit | CONTROLLER_OPEN_BUS_MASK
    }

    pub(crate) fn consume_controller_read(&mut self, player: Player) {
        if !self.controller_strobe {
            let state = &mut self.controllers[player.index()];
            state.shift = (state.shift >> 1) | 0x80;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_correct_bit_mask_for_all_buttons() {
        assert_eq!(Button::A.bit_mask(), 0b0000_0001);
        assert_eq!(Button::B.bit_mask(), 0b0000_0010);
        assert_eq!(Button::Select.bit_mask(), 0b0000_0100);
        assert_eq!(Button::Start.bit_mask(), 0b0000_1000);
        assert_eq!(Button::Up.bit_mask(), 0b0001_0000);
        assert_eq!(Button::Down.bit_mask(), 0b0010_0000);
        assert_eq!(Button::Left.bit_mask(), 0b0100_0000);
        assert_eq!(Button::Right.bit_mask(), 0b1000_0000);
    }

    #[test]
    fn should_return_correct_index_for_players() {
        assert_eq!(Player::One.index(), 0);
        assert_eq!(Player::Two.index(), 1);
    }
}
