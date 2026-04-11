use gilrs::GamepadId;
use nes_core::{Button, Command, NesCore};
use winit::event::VirtualKeyCode;
use nes_desktop::app::map_key_event_to_button_bit;
use crate::map_virtual_keycode;

pub(crate) const GAMEPAD_AXIS_THRESHOLD: f32 = 0.5;
pub(crate) const CONTROLLER_BUTTONS: [Button; 8] = [
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
pub(crate) struct GamepadSnapshot {
    pub(crate) connected: bool,
    pub(crate) south_pressed: bool,
    pub(crate) east_pressed: bool,
    pub(crate) west_pressed: bool,
    pub(crate) north_pressed: bool,
    pub(crate) select_pressed: bool,
    pub(crate) start_pressed: bool,
    pub(crate) dpad_up_pressed: bool,
    pub(crate) dpad_down_pressed: bool,
    pub(crate) dpad_left_pressed: bool,
    pub(crate) dpad_right_pressed: bool,
    pub(crate) left_x: f32,
    pub(crate) left_y: f32,
}

pub(crate) fn connected_gamepad_ids(gamepads: impl IntoIterator<Item = (GamepadId, bool)>) -> Vec<GamepadId> {
    gamepads
        .into_iter()
        .filter_map(|(id, connected)| connected.then_some(id))
        .collect()
}

pub(crate) fn select_active_gamepad_ids(
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

pub(crate) fn gamepad_snapshot_to_bits(snapshot: GamepadSnapshot) -> u8 {
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
pub(crate) fn controller_state_delta_for_player(
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

pub(crate) fn gamepad_assignments_changed(
    next: [Option<GamepadId>; 2],
    current: [Option<GamepadId>; 2],
) -> bool {
    next != current
}

pub(crate) fn gamepad_slot_changed(
    next: [Option<GamepadId>; 2],
    current: [Option<GamepadId>; 2],
    player: usize,
) -> bool {
    next[player] != current[player]
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

pub(crate) fn update_button_bits(current: u8, mask: u8, pressed: bool) -> u8 {
    if pressed {
        current | mask
    } else {
        current & !mask
    }
}

pub(crate) fn track_keyboard_bits_for_key(key: VirtualKeyCode, pressed: bool, keyboard_bits: &mut u8, map_virtual_keycode: fn(VirtualKeyCode) -> Option<&'static str>) {
    if let Some(key_code) = map_virtual_keycode(key)
        && let Some(mask) = map_key_event_to_button_bit(key_code)
    {
        *keyboard_bits = update_button_bits(*keyboard_bits, mask, pressed);
    }
}

pub(crate) fn merge_local_input_bits(keyboard_bits: u8, local_gamepad_bits: u8) -> u8 {
    keyboard_bits | local_gamepad_bits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_gamepad_id(raw: usize) -> GamepadId {
        unsafe { std::mem::transmute(raw) }
    }

    #[test]
    fn gamepad_assignment_helpers_detect_global_and_slot_level_changes() {
        let none = [None, None];
        assert!(!gamepad_assignments_changed(none, none));
        assert!(!gamepad_slot_changed(none, none, 0));
        assert!(!gamepad_slot_changed(none, none, 1));

        let next = [Some(fake_gamepad_id(1)), None];
        let current = [None, Some(fake_gamepad_id(2))];
        assert!(gamepad_assignments_changed(next, current));
        assert!(gamepad_slot_changed(next, current, 0));
        assert!(gamepad_slot_changed(next, current, 1));
    }

    #[test]
    fn gamepad_source_helpers_select_connected_ids_without_duplicates() {
        let id1 = fake_gamepad_id(1);
        let id2 = fake_gamepad_id(2);
        let id3 = fake_gamepad_id(3);
        let connected = connected_gamepad_ids(vec![(id1, true), (id2, false), (id3, true)]);
        assert_eq!(connected, vec![id1, id3]);

        let next = select_active_gamepad_ids(&connected, [Some(id1), Some(id2)]);
        assert_eq!(next, [Some(id1), Some(id3)]);

        let deduped = select_active_gamepad_ids(&connected, [Some(id3), Some(id3)]);
        assert_eq!(deduped, [Some(id3), Some(id1)]);
    }

    #[test]
    fn gamepad_sampling_helpers_map_buttons_and_axis_thresholds() {
        let bits = gamepad_snapshot_to_bits(GamepadSnapshot {
            connected: true,
            east_pressed: true,
            north_pressed: true,
            select_pressed: true,
            start_pressed: true,
            dpad_down_pressed: true,
            dpad_right_pressed: true,
            left_x: -0.75,
            left_y: -0.75,
            ..GamepadSnapshot::default()
        });
        let expected = Button::A.bit_mask()
            | Button::B.bit_mask()
            | Button::Select.bit_mask()
            | Button::Start.bit_mask()
            | Button::Up.bit_mask()
            | Button::Down.bit_mask()
            | Button::Left.bit_mask()
            | Button::Right.bit_mask();
        assert_eq!(bits, expected);

        let boundary_bits = gamepad_snapshot_to_bits(GamepadSnapshot {
            connected: true,
            left_x: GAMEPAD_AXIS_THRESHOLD,
            left_y: -GAMEPAD_AXIS_THRESHOLD,
            ..GamepadSnapshot::default()
        });
        assert_eq!(
            boundary_bits,
            Button::Up.bit_mask() | Button::Right.bit_mask()
        );

        let neutral_axis_bits = gamepad_snapshot_to_bits(GamepadSnapshot {
            connected: true,
            left_x: 0.0,
            left_y: GAMEPAD_AXIS_THRESHOLD * 0.5,
            ..GamepadSnapshot::default()
        });
        assert_eq!(neutral_axis_bits, 0);

        assert_eq!(
            gamepad_snapshot_to_bits(GamepadSnapshot {
                connected: false,
                east_pressed: true,
                left_x: 1.0,
                left_y: -1.0,
                ..GamepadSnapshot::default()
            }),
            0
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

        // mock map_virtual_keycode for test
        fn map_virtual_keycode(key: VirtualKeyCode) -> Option<&'static str> {
            match key {
                VirtualKeyCode::Z => Some("KeyZ"),
                VirtualKeyCode::F5 => None,
                _ => None,
            }
        }

        track_keyboard_bits_for_key(VirtualKeyCode::Z, true, &mut keyboard_bits, map_virtual_keycode);
        assert_eq!(keyboard_bits, Button::A.bit_mask());

        track_keyboard_bits_for_key(VirtualKeyCode::F5, true, &mut keyboard_bits, map_virtual_keycode);
        assert_eq!(
            keyboard_bits,
            Button::A.bit_mask(),
            "manual save hotkey must not alter controller state"
        );

        track_keyboard_bits_for_key(VirtualKeyCode::Z, false, &mut keyboard_bits, map_virtual_keycode);
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
