import re

with open("crates/nes-desktop/src/main.rs", "r") as f:
    code = f.read()

# Add test for reset_play_session
test_code = """
    #[test]
    fn reset_play_session_resets_components() {
        use nes_core::NesCore;
        use crate::args::{RuntimeConfig, StepMode};
        use crate::audio::{AudioOutput, AudioProvider};
        use crate::manual_state::SAVE_SLOT_COUNT;
        use crate::metrics::PerfMetrics;
        use crate::overlay::OverlayModel;
        use crate::session_cheats::SessionCheats;
        use crate::{AppContext, LoadedRomSession, reset_play_session};
        use nes_rewind::{TimeMachine, TimeMachineConfig};

        let mut core = NesCore::new();
        let mut session = LoadedRomSession {
            rom_path: std::path::PathBuf::from("test.nes"),
            rom_hash: "hash".to_string(),
            info: nes_core::RomLoadInfo {
                mapper_id: 0,
                prg_rom_bytes: 0,
                reset_pc: 0,
            },
            cheats: crate::overlay::OverlayCheatSummary {
                total_cheats: 0,
                enabled_cheats: 0,
            },
            slots: vec![],
        };
        let mut session_cheats = SessionCheats::new();
        let mut overlay = OverlayModel::new(SAVE_SLOT_COUNT);
        let runtime = RuntimeConfig {
            rom_path: "test.nes".to_string(),
            loaded_config_path: Some(std::path::PathBuf::from("config.toml")),
            step_mode: StepMode::Frame,
            audio_enabled: true,
            cheat_codes: vec![],
            rta: None,
            netplay: None,
            window_scale: 2,
            trace_every_frames: 0,
            metrics_enabled: false,
            metrics_every_frames: 0,
            capture: None,
            mcp_enabled: false,
            mcp_bind_addr: "".to_string(),
            #[cfg(feature = "nova")]
            auto_player_enabled: true,
        };
        let mut time_machine = TimeMachine::new(TimeMachineConfig::default());
        let mut rewind_held = true;
        let mut metrics = PerfMetrics::new(false, 0, 0);
        let mut gamepad_bits = [0, 0];
        let event_loop = winit::event_loop::EventLoopBuilder::new().build();
        let window = winit::window::WindowBuilder::new().build(&event_loop).unwrap();
        let mut rta_manager = None;
        let audio_provider = AudioProvider::new();
        let audio_output = audio_provider.get_output();

        let mut ctx = AppContext {
            core: &mut core,
            session: &mut session,
            session_cheats: &mut session_cheats,
            overlay: &mut overlay,
            rollback_enabled: false,
            runtime: &runtime,
            audio_output: Some(&audio_output),
            time_machine: &mut time_machine,
            rewind_held: &mut rewind_held,
            metrics: &mut metrics,
            keyboard_bits: 0,
            gamepad_bits: &mut gamepad_bits,
            window: &window,
            rta_manager: &mut rta_manager,
            frame_index: 0,
        };

        reset_play_session(&mut ctx);

        assert!(!*ctx.rewind_held);
        assert_eq!(ctx.time_machine.frame_count(), 1); // 1 initial recorded frame
    }
"""

new_code = code.replace("mod tests {", "mod tests {" + test_code)

with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(new_code)
