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
    use gilrs::GamepadId;
    use nes_core::{Button, Command, Player};

    fn fake_gamepad_id(raw: usize) -> GamepadId {
        unsafe { std::mem::transmute::<usize, GamepadId>(raw) }
    }

    #[test]
    fn should_return_only_connected_gamepad_ids() {
        let id0 = fake_gamepad_id(0);
        let id1 = fake_gamepad_id(1);
        let id2 = fake_gamepad_id(2);

        let gamepads = vec![(id0, true), (id1, false), (id2, true)];
        let connected = connected_gamepad_ids(gamepads);

        assert_eq!(connected.len(), 2, "Expected exactly two connected gamepads");
        assert!(connected.contains(&id0), "Expected connected gamepad 0 to be present");
        assert!(connected.contains(&id2), "Expected connected gamepad 2 to be present");
    }

    #[test]
    fn should_select_active_gamepad_ids_and_fill_slots() {
        let id0 = fake_gamepad_id(0);
        let id1 = fake_gamepad_id(1);
        let id2 = fake_gamepad_id(2);

        let connected = vec![id0, id1];
        let current = [None, None];

        let next = select_active_gamepad_ids(&connected, current);
        assert_eq!(next[0], Some(id0), "Expected slot 0 to be filled by first gamepad");
        assert_eq!(next[1], Some(id1), "Expected slot 1 to be filled by second gamepad");

        let connected2 = vec![id0, id2];
        let current2 = [Some(id0), Some(id1)];

        let next2 = select_active_gamepad_ids(&connected2, current2);
        assert_eq!(next2[0], Some(id0), "Expected slot 0 to retain previously assigned active gamepad");
        assert_eq!(next2[1], Some(id2), "Expected slot 1 to replace disconnected gamepad with newly active one");
    }

    #[test]
    fn should_map_gamepad_snapshot_to_correct_nes_bits() {
        let mut snapshot = GamepadSnapshot::default();
        assert_eq!(gamepad_snapshot_to_bits(snapshot), 0, "Expected empty snapshot to return 0 mask");

        snapshot.connected = true;
        snapshot.south_pressed = true;
        snapshot.start_pressed = true;

        let bits = gamepad_snapshot_to_bits(snapshot);
        assert_ne!(bits & Button::A.bit_mask(), 0, "Expected A button to be set");
        assert_ne!(bits & Button::Start.bit_mask(), 0, "Expected Start button to be set");
        assert_eq!(bits & Button::B.bit_mask(), 0, "Expected B button to not be set");

        snapshot.south_pressed = false;
        snapshot.east_pressed = true;
        let bits2 = gamepad_snapshot_to_bits(snapshot);
        assert_ne!(bits2 & Button::A.bit_mask(), 0, "Expected East face button to map to A");

        snapshot.west_pressed = true;
        let bits3 = gamepad_snapshot_to_bits(snapshot);
        assert_ne!(bits3 & Button::B.bit_mask(), 0, "Expected West face button to map to B");

        snapshot.north_pressed = true;
        let bits4 = gamepad_snapshot_to_bits(snapshot);
        assert_ne!(bits4 & Button::B.bit_mask(), 0, "Expected North face button to map to B");

        snapshot.select_pressed = true;
        let bits5 = gamepad_snapshot_to_bits(snapshot);
        assert_ne!(bits5 & Button::Select.bit_mask(), 0, "Expected Select button to be set");

        snapshot.dpad_up_pressed = true;
        snapshot.dpad_down_pressed = true;
        snapshot.dpad_left_pressed = true;
        snapshot.dpad_right_pressed = true;
        let bits6 = gamepad_snapshot_to_bits(snapshot);
        assert_ne!(bits6 & Button::Up.bit_mask(), 0, "Expected Up button to be set via dpad");
        assert_ne!(bits6 & Button::Down.bit_mask(), 0, "Expected Down button to be set via dpad");
        assert_ne!(bits6 & Button::Left.bit_mask(), 0, "Expected Left button to be set via dpad");
        assert_ne!(bits6 & Button::Right.bit_mask(), 0, "Expected Right button to be set via dpad");

        snapshot.dpad_up_pressed = false;
        snapshot.dpad_down_pressed = false;
        snapshot.dpad_left_pressed = false;
        snapshot.dpad_right_pressed = false;

        snapshot.left_y = -0.6;
        let bits7 = gamepad_snapshot_to_bits(snapshot);
        assert_ne!(bits7 & Button::Up.bit_mask(), 0, "Expected Up button to be set via stick threshold");

        snapshot.left_y = 0.6;
        let bits8 = gamepad_snapshot_to_bits(snapshot);
        assert_ne!(bits8 & Button::Down.bit_mask(), 0, "Expected Down button to be set via stick threshold");

        snapshot.left_y = 0.0;
        snapshot.left_x = -0.6;
        let bits9 = gamepad_snapshot_to_bits(snapshot);
        assert_ne!(bits9 & Button::Left.bit_mask(), 0, "Expected Left button to be set via stick threshold");

        snapshot.left_x = 0.6;
        let bits10 = gamepad_snapshot_to_bits(snapshot);
        assert_ne!(bits10 & Button::Right.bit_mask(), 0, "Expected Right button to be set via stick threshold");
    }

    #[test]
    fn should_emit_press_and_release_commands_for_state_deltas() {
        let mut cmds = controller_state_delta_for_player(0, Button::A.bit_mask(), Player::One);
        assert_eq!(cmds.next(), Some(Command::PressButton(Button::A)), "Expected Player One A button press command");
        assert_eq!(cmds.next(), None, "Expected no additional commands");

        let mut cmds2 = controller_state_delta_for_player(Button::A.bit_mask(), 0, Player::One);
        assert_eq!(cmds2.next(), Some(Command::ReleaseButton(Button::A)), "Expected Player One A button release command");
        assert_eq!(cmds2.next(), None, "Expected no additional commands");

        let mut cmds3 = controller_state_delta_for_player(0, Button::B.bit_mask(), Player::Two);
        assert_eq!(cmds3.next(), Some(Command::PressButton2(Button::B)), "Expected Player Two B button press command");
        assert_eq!(cmds3.next(), None, "Expected no additional commands");

        let mut cmds4 = controller_state_delta_for_player(Button::B.bit_mask(), 0, Player::Two);
        assert_eq!(cmds4.next(), Some(Command::ReleaseButton2(Button::B)), "Expected Player Two B button release command");
        assert_eq!(cmds4.next(), None, "Expected no additional commands");
    }
}
