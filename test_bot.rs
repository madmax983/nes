use nes_core::experimental::spatial_bot::{SpatialBot, BotRule};
use nes_core::Button;

fn main() {
    let mut bot = SpatialBot::new();
    bot.add_rule(1, Button::B, 0);
}
