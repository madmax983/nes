use gilrs::{Axis, Button as GilrsButton, Gamepad, GamepadId};
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

#[cfg(not(tarpaulin_include))]
impl From<&Gamepad<'_>> for GamepadSnapshot {
    fn from(gamepad: &Gamepad<'_>) -> Self {
        Self {
            connected: gamepad.is_connected(),
            south_pressed: gamepad.is_pressed(GilrsButton::South),
            east_pressed: gamepad.is_pressed(GilrsButton::East),
            west_pressed: gamepad.is_pressed(GilrsButton::West),
            north_pressed: gamepad.is_pressed(GilrsButton::North),
            select_pressed: gamepad.is_pressed(GilrsButton::Select),
            start_pressed: gamepad.is_pressed(GilrsButton::Start),
            dpad_up_pressed: gamepad.is_pressed(GilrsButton::DPadUp),
            dpad_down_pressed: gamepad.is_pressed(GilrsButton::DPadDown),
            dpad_left_pressed: gamepad.is_pressed(GilrsButton::DPadLeft),
            dpad_right_pressed: gamepad.is_pressed(GilrsButton::DPadRight),
            left_x: gamepad.value(Axis::LeftStickX),
            left_y: gamepad.value(Axis::LeftStickY),
        }
    }
}

/// Filters a list of gamepads to return only those that are currently connected.
///
/// This is a convenience helper used when polling gamepads from `gilrs` to
/// easily collect the IDs of active controllers.
///
/// # Examples
/// ```
/// use gilrs::GamepadId;
/// use nes_desktop::gamepad::connected_gamepad_ids;
///
/// // Unsafe is used here only to mock Gilrs' opaque `GamepadId` for the doctest.
/// let id0 = unsafe { std::mem::transmute::<usize, GamepadId>(0) };
/// let id1 = unsafe { std::mem::transmute::<usize, GamepadId>(1) };
/// let id2 = unsafe { std::mem::transmute::<usize, GamepadId>(2) };
///
/// let gamepads = vec![(id0, true), (id1, false), (id2, true)];
/// let connected = connected_gamepad_ids(gamepads);
///
/// assert_eq!(connected.len(), 2);
/// assert!(connected.contains(&id0));
/// assert!(connected.contains(&id2));
/// ```
pub fn connected_gamepad_ids(
    gamepads: impl IntoIterator<Item = (GamepadId, bool)>,
) -> Vec<GamepadId> {
    gamepads
        .into_iter()
        .filter_map(|(id, connected)| connected.then_some(id))
        .collect()
}

/// Selects up to two active gamepads to assign to Player 1 and Player 2.
///
/// This function prioritizes keeping currently assigned gamepads in their
/// respective slots if they are still connected. If a slot is empty and
/// there are unassigned connected gamepads available, it will fill the slot.
///
/// # Examples
/// ```
/// use gilrs::GamepadId;
/// use nes_desktop::gamepad::select_active_gamepad_ids;
///
/// let id0 = unsafe { std::mem::transmute::<usize, GamepadId>(0) };
/// let id1 = unsafe { std::mem::transmute::<usize, GamepadId>(1) };
///
/// let connected = vec![id0, id1];
/// // Currently, no controllers are assigned.
/// let current = [None, None];
///
/// let next = select_active_gamepad_ids(&connected, current);
/// assert_eq!(next[0], Some(id0));
/// assert_eq!(next[1], Some(id1));
/// ```
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

/// Converts a raw `GamepadSnapshot` into an 8-bit NES button mask.
///
/// This mapping translates modern controller inputs (like analog sticks and face buttons)
/// into the standard 8 buttons recognized by the NES: A, B, Select, Start, Up, Down, Left, Right.
///
/// It correctly handles mapping both South/East to A and West/North to B, accommodating both
/// Xbox-style and Switch-style layouts. Stick thresholds are also applied for d-pad behavior.
///
/// # Examples
/// ```
/// use nes_core::Button;
/// use nes_desktop::gamepad::{GamepadSnapshot, gamepad_snapshot_to_bits};
///
/// let mut snapshot = GamepadSnapshot::default();
/// snapshot.connected = true;
/// snapshot.south_pressed = true; // Typically 'A' or 'B' depending on layout, mapped to NES 'A'
/// snapshot.start_pressed = true;
///
/// let bits = gamepad_snapshot_to_bits(snapshot);
///
/// // Assert the correct NES bit masks are set
/// assert_ne!(bits & Button::A.bit_mask(), 0);
/// assert_ne!(bits & Button::Start.bit_mask(), 0);
/// assert_eq!(bits & Button::B.bit_mask(), 0);
/// ```
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

#[cfg(test)]
mod tests {
    use super::*;

    // Note: It's hard to mock `gilrs::Gamepad` directly for unit tests because it's tightly coupled to the gilrs state machine.
    // In this codebase, since we only extracted existing logic, it might be that `gilrs::Gamepad` is mocked elsewhere or it's just considered integration boundary.
    // Let's check how gamepad is mocked elsewhere in this project or if there's an easier way to test this.
}
