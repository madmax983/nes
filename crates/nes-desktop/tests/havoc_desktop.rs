use proptest::prelude::*;

proptest! {
    #[test]
    #[ignore = "havoc target"]
    fn proptest_actions_action_from_menu_id(s in ".*") {
        let _ = nes_desktop::actions::action_from_menu_id(&s);
    }

    #[test]
    #[ignore = "havoc target"]
    fn proptest_menu_action_from_menu_event_id(s in ".*") {
        let _ = nes_desktop::menu::action_from_menu_event_id(&s);
    }
}

proptest! {
    #[test]
    #[ignore = "havoc target"]
    fn proptest_overlay_status_message(s in ".*") {
        let mut overlay = nes_desktop::overlay::OverlayModel::new(4);
        overlay.set_status_message(&s);
        let _ = overlay.status_message();
    }
}

proptest! {
    #[test]
    #[ignore = "havoc target"]
    fn proptest_manual_state_slot_path_for_rom(s in ".*", slot in any::<u8>()) {
        let _ = nes_desktop::manual_state::slot_path_for_rom(&std::path::PathBuf::from("test"), &s, slot);
    }
}

proptest! {
    #[test]
    #[ignore = "havoc target"]
    fn proptest_app_map_key_event_to_button_bit(s in ".*") {
        let _ = nes_desktop::app::map_key_event_to_button_bit(&s);
    }
}
