use gilrs::GamepadId;
use nes_core::{Button, Command, NesCore};
use nes_desktop::app::map_key_event_to_button_bit;
use winit::event::VirtualKeyCode;

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

pub(crate) fn update_button_bits(current: u8, mask: u8, pressed: bool) -> u8 {
    if pressed {
        current | mask
    } else {
        current & !mask
    }
}

pub(crate) fn track_keyboard_bits_for_key(
    key: VirtualKeyCode,
    pressed: bool,
    keyboard_bits: &mut u8,
) {
    if let Some(key_code) = crate::input::map_virtual_keycode(key)
        && let Some(mask) = map_key_event_to_button_bit(key_code)
    {
        *keyboard_bits = update_button_bits(*keyboard_bits, mask, pressed);
    }
}

pub(crate) fn merge_local_input_bits(keyboard_bits: u8, local_gamepad_bits: u8) -> u8 {
    keyboard_bits | local_gamepad_bits
}

pub(crate) fn release_all_buttons(core: &mut NesCore) {
    for &button in &CONTROLLER_BUTTONS {
        let _ = core.execute(Command::ReleaseButton(button));
        let _ = core.execute(Command::ReleaseButton2(button));
    }
}

pub(crate) fn resync_restored_inputs(
    core: &mut NesCore,
    keyboard_bits: u8,
    gamepad_bits: &mut [u8; 2],
) -> Result<(), String> {
    release_all_buttons(core);
    *gamepad_bits = [0; 2];
    apply_gamepad_delta_commands(core, 0, keyboard_bits, nes_core::Player::One)
}

pub(crate) fn is_player_two_slot(player_index: usize) -> bool {
    player_index == 1
}

pub(crate) fn apply_gamepad_delta_commands(
    core: &mut NesCore,
    previous_bits: u8,
    next_bits: u8,
    player: nes_core::Player,
) -> Result<(), String> {
    for command in controller_state_delta_for_player(previous_bits, next_bits, player) {
        core.execute(command)
            .map_err(|err| format!("Gamepad command failed: {err}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nes_core::NesCore;
    use winit::event::VirtualKeyCode;

    #[test]
    fn gamepad_helper_primitives_cover_player_flags_and_local_input_bits() {
        assert!(!is_player_two_slot(0));
        assert!(is_player_two_slot(1));
        assert_eq!(
            merge_local_input_bits(0b0000_0011, 0b0000_0101),
            0b0000_0111
        );
    }

    #[test]
    fn update_button_bits_sets_and_clears_masks() {
        let with_a = update_button_bits(0, Button::A.bit_mask(), true);
        assert_eq!(with_a, Button::A.bit_mask());
        // Pressing an already-set bit should be idempotent.
        assert_eq!(
            update_button_bits(with_a, Button::A.bit_mask(), true),
            Button::A.bit_mask()
        );
        let with_ab = update_button_bits(with_a, Button::B.bit_mask(), true);
        assert_eq!(with_ab, Button::A.bit_mask() | Button::B.bit_mask());
        let cleared_a = update_button_bits(with_ab, Button::A.bit_mask(), false);
        assert_eq!(cleared_a, Button::B.bit_mask());
    }

    #[test]
    fn track_keyboard_bits_for_key_updates_controller_bits_and_ignores_hotkeys() {
        let mut keyboard_bits = 0_u8;

        track_keyboard_bits_for_key(VirtualKeyCode::Z, true, &mut keyboard_bits);
        assert_eq!(keyboard_bits, Button::A.bit_mask());

        track_keyboard_bits_for_key(VirtualKeyCode::F5, true, &mut keyboard_bits);
        assert_eq!(
            keyboard_bits,
            Button::A.bit_mask(),
            "manual save hotkey must not alter controller state"
        );

        track_keyboard_bits_for_key(VirtualKeyCode::Z, false, &mut keyboard_bits);
        assert_eq!(keyboard_bits, 0);
    }

    #[test]
    fn resync_restored_inputs_reapplies_keyboard_and_resets_gamepad_cache() {
        let mut core = NesCore::new();
        let mut gamepad_bits = [Button::Right.bit_mask(), Button::Start.bit_mask()];

        resync_restored_inputs(&mut core, Button::A.bit_mask(), &mut gamepad_bits)
            .expect("restored inputs should resync");

        assert_eq!(
            core.controller_bits(),
            Button::A.bit_mask(),
            "held keyboard input should be re-applied immediately"
        );
        assert_eq!(
            core.controller2_bits(),
            0,
            "player-2 gamepad state should be cleared until the next poll replays it"
        );
        assert_eq!(
            gamepad_bits,
            [0, 0],
            "gamepad cache must reset so held pads generate deltas on the next poll"
        );
    }

    #[test]
    fn apply_gamepad_delta_commands_updates_controller_bits() {
        let mut core = NesCore::new();
        apply_gamepad_delta_commands(
            &mut core,
            0,
            Button::A.bit_mask() | Button::Right.bit_mask(),
            nes_core::Player::One,
        )
        .expect("applying player-1 gamepad delta should succeed");
        assert_eq!(
            core.controller_bits(),
            Button::A.bit_mask() | Button::Right.bit_mask()
        );

        apply_gamepad_delta_commands(
            &mut core,
            Button::A.bit_mask() | Button::Right.bit_mask(),
            Button::Right.bit_mask(),
            nes_core::Player::One,
        )
        .expect("releasing one player-1 button should succeed");
        assert_eq!(core.controller_bits(), Button::Right.bit_mask());

        apply_gamepad_delta_commands(
            &mut core,
            0,
            Button::Start.bit_mask(),
            nes_core::Player::Two,
        )
        .expect("applying player-2 gamepad delta should succeed");
        assert_eq!(core.controller2_bits(), Button::Start.bit_mask());
    }

    #[test]
    fn controller_state_delta_emits_press_and_release() {
        let press: Vec<_> = controller_state_delta_for_player(
            0,
            Button::A.bit_mask() | Button::Right.bit_mask(),
            nes_core::Player::One,
        )
        .collect();
        assert_eq!(
            press,
            vec![
                Command::PressButton(Button::A),
                Command::PressButton(Button::Right)
            ]
        );

        let release: Vec<_> = controller_state_delta_for_player(
            Button::A.bit_mask() | Button::B.bit_mask(),
            Button::B.bit_mask(),
            nes_core::Player::One,
        )
        .collect();
        assert_eq!(release, vec![Command::ReleaseButton(Button::A)]);
    }

    #[test]
    fn controller_state_delta_for_player2_uses_player2_commands() {
        let press: Vec<_> =
            controller_state_delta_for_player(0, Button::A.bit_mask(), nes_core::Player::Two)
                .collect();
        assert_eq!(press, vec![Command::PressButton2(Button::A)]);

        let release: Vec<_> =
            controller_state_delta_for_player(Button::Start.bit_mask(), 0, nes_core::Player::Two)
                .collect();
        assert_eq!(release, vec![Command::ReleaseButton2(Button::Start)]);
    }
}
