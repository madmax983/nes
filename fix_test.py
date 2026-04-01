import re

with open("crates/nes-desktop/src/main.rs", "r") as f:
    content = f.read()

# Generate a minimal test for execute_app_action to fix coverage
test_content = """
    #[test]
    fn execute_app_action_branches_return_expected_results() {
        let mut core = NesCore::new();
        let mut session = LoadedRomSession {
            rom_hash: String::from("12345678"),
            rom_path: std::path::PathBuf::from("test.nes"),
            state_dir: std::path::PathBuf::from("states"),
        };
        let mut session_cheats = SessionCheats::new();
        let mut overlay = OverlayModel::new(5);
        let runtime = RuntimeConfig {
            rom_path: String::new(),
            loaded_config_path: None,
            step_mode: StepMode::Frame,
            audio_enabled: false,
            cheat_codes: vec![],
            mcp_enabled: false,
            mcp_bind_addr: String::new(),
            netplay_enabled: false,
            netplay_relay_addr: None,
            netplay_room: None,
            netplay_player: None,
            netplay_input_delay_frames: None,
            netplay_max_rollback_frames: None,
            netplay_hash_check_every_frames: None,
            rta_enabled: false,
            rta_profile_id: None,
            rta_profiles_dir: None,
            rta_runs_dir: None,
            rta_calibrate: false,
            auto_player_enabled: false,
            logging_save_input_log: false,
            graphics_integer_scale: false,
            graphics_theme: crate::config::ThemeName::Classic,
            graphics_crt_filter: crate::config::CrtFilter::None,
            graphics_hide_hud: false,
            metrics_enabled: false,
            metrics_every_frames: 60,
        };
        let mut time_machine = TimeMachine::new(TimeMachineConfig::default());
        let mut rewind_held = false;
        let mut metrics = PerfMetrics::new(false, 60, 0);
        let mut gamepad_bits = [0_u8; 2];
        let event_loop = winit::event_loop::EventLoop::new();
        let window = winit::window::WindowBuilder::new().build(&event_loop).unwrap();
        let mut rta_manager = None;

        let mut ctx = AppContext {
            core: &mut core,
            session: &mut session,
            session_cheats: &mut session_cheats,
            overlay: &mut overlay,
            rollback_enabled: false,
            runtime: &runtime,
            audio_output: None,
            time_machine: &mut time_machine,
            rewind_held: &mut rewind_held,
            metrics: &mut metrics,
            keyboard_bits: 0,
            gamepad_bits: &mut gamepad_bits,
            window: &window,
            rta_manager: &mut rta_manager,
            frame_index: 0,
        };

        // Quit returns true
        assert_eq!(execute_app_action(AppAction::Quit, &mut ctx), Ok(true));

        // Other actions return false
        assert_eq!(execute_app_action(AppAction::ToggleOverlay, &mut ctx), Ok(false));
        assert_eq!(execute_app_action(AppAction::Resume, &mut ctx), Ok(false));
        assert_eq!(execute_app_action(AppAction::Reset, &mut ctx), Ok(false));
    }
"""

if "fn execute_app_action_branches_return_expected_results" not in content:
    idx = content.rfind("}")
    content = content[:idx] + test_content + "\n}\n"
    with open("crates/nes-desktop/src/main.rs", "w") as f:
        f.write(content)
