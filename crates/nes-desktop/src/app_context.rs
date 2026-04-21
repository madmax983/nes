use nes_core::NesCore;
use winit::window::Window;
use crate::config::RuntimeConfig;
use crate::session::LoadedRomSession;
use nes_desktop::session_cheats::SessionCheats;
use nes_desktop::overlay::OverlayModel;
use nes_desktop::audio::AudioOutput;
use nes_rewind::worker::TimeMachine;
use crate::metrics::PerfMetrics;
use nes_desktop::rta::RtaManager;

pub(crate) struct AppContext<'a> {
    pub core: &'a mut NesCore,
    pub session: &'a mut LoadedRomSession,
    pub session_cheats: &'a mut SessionCheats,
    pub overlay: &'a mut OverlayModel,
    pub rollback_enabled: bool,
    pub runtime: &'a RuntimeConfig,
    pub audio_output: Option<&'a AudioOutput>,
    pub time_machine: &'a mut TimeMachine,
    pub rewind_held: &'a mut bool,
    pub metrics: &'a mut PerfMetrics,
    pub keyboard_bits: u8,
    pub gamepad_bits: &'a mut [u8; 2],
    pub window: &'a Window,
    pub rta_manager: &'a mut Option<RtaManager>,
    pub frame_index: u64,
}
