#![no_main]
use libfuzzer_sys::fuzz_target;
use nes_core::{NesCore, Command, Button};
use arbitrary::Arbitrary;

#[derive(Arbitrary, Debug)]
enum FuzzButton {
    A,
    B,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
}

impl Into<Button> for FuzzButton {
    fn into(self) -> Button {
        match self {
            FuzzButton::A => Button::A,
            FuzzButton::B => Button::B,
            FuzzButton::Select => Button::Select,
            FuzzButton::Start => Button::Start,
            FuzzButton::Up => Button::Up,
            FuzzButton::Down => Button::Down,
            FuzzButton::Left => Button::Left,
            FuzzButton::Right => Button::Right,
        }
    }
}

#[derive(Arbitrary, Debug)]
enum FuzzCommand {
    Pause,
    Resume,
    Reset,
    PowerCycle,
    StepCpu,
    StepScanline,
    StepFrame,
    SetControllerState(u8),
    PressButton(FuzzButton),
    ReleaseButton(FuzzButton),
    SetSpeed(u16),
}

impl Into<Command> for FuzzCommand {
    fn into(self) -> Command {
        match self {
            FuzzCommand::Pause => Command::Pause,
            FuzzCommand::Resume => Command::Resume,
            FuzzCommand::Reset => Command::Reset,
            FuzzCommand::PowerCycle => Command::PowerCycle,
            FuzzCommand::StepCpu => Command::StepCpu,
            FuzzCommand::StepScanline => Command::StepScanline,
            FuzzCommand::StepFrame => Command::StepFrame,
            FuzzCommand::SetControllerState(s) => Command::SetControllerState(s),
            FuzzCommand::PressButton(b) => Command::PressButton(b.into()),
            FuzzCommand::ReleaseButton(b) => Command::ReleaseButton(b.into()),
            FuzzCommand::SetSpeed(s) => Command::SetSpeed(s),
        }
    }
}

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    commands: Vec<FuzzCommand>,
}

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = FuzzInput::arbitrary(&mut arbitrary::Unstructured::new(data)) {
        let mut core = NesCore::new();
        for command in input.commands {
            let _ = core.execute(command.into());
        }
    }
});
