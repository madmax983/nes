use nes_ai::actions::ControlAction;
use nes_core::Button;

#[test]
fn action_ids_map_to_expected_controller_masks() {
    assert_eq!(ControlAction::Noop.controller1_bits(), 0);
    assert_eq!(
        ControlAction::Right.controller1_bits(),
        Button::Right.bit_mask()
    );
    assert_eq!(
        ControlAction::RightA.controller1_bits(),
        Button::Right.bit_mask() | Button::A.bit_mask()
    );
    assert_eq!(ControlAction::A.controller1_bits(), Button::A.bit_mask());
    assert_eq!(
        ControlAction::RightB.controller1_bits(),
        Button::Right.bit_mask() | Button::B.bit_mask()
    );
    assert_eq!(
        ControlAction::RightAB.controller1_bits(),
        Button::Right.bit_mask() | Button::A.bit_mask() | Button::B.bit_mask()
    );
}
