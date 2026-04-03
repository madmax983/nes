use gilrs::GamepadId;
use nes_core::{Button, Command};

pub const GAMEPAD_AXIS_THRESHOLD: f32 = 0.5;

pub const CONTROLLER_BUTTONS: [Button; 8] = [
    Button::A,
    Button::B,
    Button::Select,
    Button::Start,
    Button::Up,
    Button::Down,
    Button::Left,
    Button::Right,
];

#[derive(Debug, Clone, Copy, Default)]
pub struct GamepadSnapshot {
    pub connected: bool,
    pub south_pressed: bool,
    pub east_pressed: bool,
    pub west_pressed: bool,
    pub north_pressed: bool,
    pub select_pressed: bool,
    pub start_pressed: bool,
    pub dpad_up_pressed: bool,
    pub dpad_down_pressed: bool,
    pub dpad_left_pressed: bool,
    pub dpad_right_pressed: bool,
    pub left_x: f32,
    pub left_y: f32,
}

pub fn connected_gamepad_ids(
    gamepads: impl IntoIterator<Item = (GamepadId, bool)>,
) -> Vec<GamepadId> {
    gamepads
        .into_iter()
        .filter_map(|(id, connected)| connected.then_some(id))
        .collect()
}

pub fn select_active_gamepad_ids(
    connected: &[GamepadId],
    current: [Option<GamepadId>; 2],
) -> [Option<GamepadId>; 2] {
    let mut next = [None::<GamepadId>; 2];

    for player in 0..next.len() {
        if let Some(gamepad_id) = current[player]
            && connected.contains(&gamepad_id)
            && !next.contains(&Some(gamepad_id))
        {
            next[player] = Some(gamepad_id);
        }
    }

    for &gamepad_id in connected {
        if next.iter().all(|slot| *slot != Some(gamepad_id))
            && let Some(slot) = next.iter_mut().find(|slot| slot.is_none())
        {
            *slot = Some(gamepad_id);
        }
    }

    next
}

pub fn gamepad_snapshot_to_bits(snapshot: GamepadSnapshot) -> u8 {
    if !snapshot.connected {
        return 0;
    }

    let mut bits = 0_u8;
    // Keep both common face layouts usable across Xbox/Switch-style controllers.
    if snapshot.south_pressed || snapshot.east_pressed {
        bits |= Button::A.bit_mask();
    }
    if snapshot.west_pressed || snapshot.north_pressed {
        bits |= Button::B.bit_mask();
    }
    if snapshot.select_pressed {
        bits |= Button::Select.bit_mask();
    }
    if snapshot.start_pressed {
        bits |= Button::Start.bit_mask();
    }

    if snapshot.dpad_up_pressed || snapshot.left_y <= -GAMEPAD_AXIS_THRESHOLD {
        bits |= Button::Up.bit_mask();
    }
    if snapshot.dpad_down_pressed || snapshot.left_y >= GAMEPAD_AXIS_THRESHOLD {
        bits |= Button::Down.bit_mask();
    }
    if snapshot.dpad_left_pressed || snapshot.left_x <= -GAMEPAD_AXIS_THRESHOLD {
        bits |= Button::Left.bit_mask();
    }
    if snapshot.dpad_right_pressed || snapshot.left_x >= GAMEPAD_AXIS_THRESHOLD {
        bits |= Button::Right.bit_mask();
    }

    bits
}

/// **Performance optimization:** Returns an `impl Iterator` instead of `Vec<Command>`
/// to eliminate a per-frame heap allocation when processing small, bounded state changes
/// for the NES controllers.
pub fn controller_state_delta_for_player(
    previous: u8,
    current: u8,
    player: nes_core::Player,
) -> impl Iterator<Item = Command> {
    CONTROLLER_BUTTONS.into_iter().filter_map(move |button| {
        let mask = button.bit_mask();
        match (previous & mask != 0, current & mask != 0) {
            (false, true) => Some(match player {
                nes_core::Player::One => Command::PressButton(button),
                nes_core::Player::Two => Command::PressButton2(button),
            }),
            (true, false) => Some(match player {
                nes_core::Player::One => Command::ReleaseButton(button),
                nes_core::Player::Two => Command::ReleaseButton2(button),
            }),
            _ => None,
        }
    })
}
