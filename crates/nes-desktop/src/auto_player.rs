#[cfg(feature = "nova")]
use nes_core::{Button, Command, NesCore};
#[cfg(feature = "nova")]
use rand::Rng;

/// A simplistic, automated player that periodically injects random inputs into the emulator.
///
/// This is used primarily in experimental/nova builds to generate chaos or background activity
/// without requiring human interaction.
///
/// ## Examples
///
/// ```
/// use nes_core::NesCore;
/// use nes_desktop::auto_player::AutoPlayer;
///
/// let mut core = NesCore::new();
/// let mut auto_player = AutoPlayer::new();
///
/// // Step the emulator and allow the auto player to generate inputs
/// auto_player.step(&mut core);
/// ```
#[cfg(feature = "nova")]
pub struct AutoPlayer {
    frame_counter: u64,
    current_buttons: u8,
}

#[cfg(feature = "nova")]
impl AutoPlayer {
    /// Creates a new `AutoPlayer` with internal frame counters reset.
    pub fn new() -> Self {
        Self {
            frame_counter: 0,
            current_buttons: 0,
        }
    }

    /// Updates the auto player's internal state and injects input commands into the `NesCore`
    /// if an input interval has elapsed.
    pub fn step(&mut self, core: &mut NesCore) {
        self.frame_counter = self.frame_counter.wrapping_add(1);

        // Update inputs every 10 frames
        #[allow(clippy::manual_is_multiple_of)]
        if self.frame_counter % 10 == 0 {
            let mut rng = rand::thread_rng();
            let new_buttons: u8 = rng.r#gen(); // Randomly press any buttons

            // Compare new_buttons with current_buttons to send Press/Release commands
            for button in [
                Button::A,
                Button::B,
                Button::Select,
                Button::Start,
                Button::Up,
                Button::Down,
                Button::Left,
                Button::Right,
            ] {
                let mask = button.bit_mask();
                let was_pressed = (self.current_buttons & mask) != 0;
                let is_pressed = (new_buttons & mask) != 0;

                if is_pressed && !was_pressed {
                    let _ = core.execute(Command::PressButton(button));
                } else if !is_pressed && was_pressed {
                    let _ = core.execute(Command::ReleaseButton(button));
                }
            }

            self.current_buttons = new_buttons;
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn auto_player_injects_inputs() {
        let mut core = NesCore::new();
        let mut auto_player = AutoPlayer::new();

        let initial_buttons = core.controller_bits();
        assert_eq!(initial_buttons, 0);

        // Step auto player multiple times to trigger an input generation
        for _ in 0..10 {
            auto_player.step(&mut core);
        }

        // Auto player uses random inputs, so it could technically generate 0,
        // but it's highly likely to generate a non-zero input over a few 10-frame intervals.
        // We'll run it a bit more to ensure an input is generated.
        let mut buttons_pressed = core.controller_bits();
        for _ in 0..100 {
            auto_player.step(&mut core);
            buttons_pressed |= core.controller_bits();
        }

        // At least one button should have been pressed over 11 input cycles
        assert!(buttons_pressed != 0);
    }
}
