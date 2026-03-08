//! Story Demo (Nova Experiment)
//!
//! This example demonstrates the `NarrativeGenerator` feature.
//! It drives the NES core with synthetic inputs and prints the resulting story.

use nes_core::experimental::narrative::NarrativeGenerator;
use nes_core::{Button, Command, NesCore};

fn main() {
    println!("🌟 Nova: Narrative Generator Demo");
    println!("=================================");

    let mut core = NesCore::new();
    let mut generator = NarrativeGenerator::new();

    // 1. Initial observation
    if let Some(event) = generator.observe(&core) {
        println!("{}", event);
    }

    // 2. Change speed
    core.execute(Command::SetSpeed(2000)).unwrap();
    if let Some(event) = generator.observe(&core) {
        println!("{}", event);
    }

    // 3. Press a button
    core.execute(Command::PressButton(Button::A)).unwrap();
    if let Some(event) = generator.observe(&core) {
        println!("{}", event);
    }

    // 4. Press another button
    core.execute(Command::PressButton(Button::B)).unwrap();
    if let Some(event) = generator.observe(&core) {
        println!("{}", event);
    }

    println!("=================================");
    println!("Demo complete.");
}
