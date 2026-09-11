//! Active-low GPIO buttons with internal pull-ups, polled into the NES
//! controller bitfields the core expects.

use esp_hal::gpio::{Input, InputConfig, Pull};
use nes_embassy::{buttons, Button};

use crate::board::{self, steal_pin};

/// Player-1 pad. (Player 2 is unpopulated on the reference board.)
pub struct Pad {
    pins: [Input<'static>; 8],
}

impl Pad {
    /// Builds the reference player-1 pad from the board pin map.
    pub fn player_one() -> Self {
        let config = InputConfig::default().with_pull(Pull::Up);
        let numbers = [
            board::BTN_A,
            board::BTN_B,
            board::BTN_SELECT,
            board::BTN_START,
            board::BTN_UP,
            board::BTN_DOWN,
            board::BTN_LEFT,
            board::BTN_RIGHT,
        ];
        Self {
            pins: numbers.map(|n| Input::new(steal_pin(n), config)),
        }
    }

    /// Samples all buttons, returning the player-1 controller bitfield
    /// (A, B, Select, Start, Up, Down, Left, Right in bits 0-7).
    pub fn poll(&self) -> u8 {
        const MAPPING: [Button; 8] = [
            Button::A,
            Button::B,
            Button::Select,
            Button::Start,
            Button::Up,
            Button::Down,
            Button::Left,
            Button::Right,
        ];
        let mut pressed = [Button::A; 8];
        let mut count = 0;
        for (pin, button) in self.pins.iter().zip(MAPPING) {
            // Active-low: a grounded pin means "pressed".
            if pin.is_low() {
                pressed[count] = button;
                count += 1;
            }
        }
        buttons(&pressed[..count])
    }
}
