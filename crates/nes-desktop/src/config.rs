use std::env;
use std::path::PathBuf;

use crate::netplay::NetplayRuntimeConfig;
use nes_config::{
    DEFAULT_CONFIG_PATH, NesConfig, StepModeConfig, normalize_nonzero_u32, normalize_nonzero_u64,
    parse_config_path_arg,
};
use nes_desktop::args::parse_runtime_args;
use nes_desktop::rta::{DEFAULT_RTA_PROFILES_DIR, DEFAULT_RTA_RUNS_DIR, RtaRuntimeConfig};

pub const DEFAULT_CPU_STEPS_PER_FRAME: u32 = 10_000;
pub const DEFAULT_WINDOW_SCALE: u32 = 3;
pub const DEFAULT_TRACE_EVERY_FRAMES: u64 = 0;
pub const DEFAULT_CAPTURE_EVERY_FRAMES: u64 = 1;

pub fn netplay_feature_enabled(runtime_flag: bool, config_flag: bool) -> bool {
    runtime_flag || config_flag
}

pub struct RuntimeConfig {
    pub rom_path: String,
    pub cheat_codes: Vec<String>,
    pub window_scale: u32,
    pub step_mode: StepMode,
    pub audio_enabled: bool,
    pub trace_every_frames: u64,
    pub metrics_enabled: bool,
    pub metrics_every_frames: u64,
    pub capture: Option<CaptureConfig>,
    pub loaded_config_path: Option<PathBuf>,
    pub mcp_enabled: bool,
    pub mcp_bind_addr: String,
    pub netplay: Option<NetplayRuntimeConfig>,
    pub rta: Option<RtaRuntimeConfig>,
    #[cfg(feature = "nova")]
    pub auto_player_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    CpuBudget(u32),
    Frame,
}

#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub path_template: String,
    pub every_n_frames: u64,
}

pub fn capture_config_from_parts(
    path_template: Option<String>,
    every_n_frames: u64,
) -> Option<CaptureConfig> {
    let template = path_template?;
    if template.trim().is_empty() {
        return None;
    }
    Some(CaptureConfig {
        path_template: template,
        every_n_frames: normalize_nonzero_u64(every_n_frames, DEFAULT_CAPTURE_EVERY_FRAMES),
    })
}

pub fn capture_path_for_frame(template: &str, frame: u64) -> String {
    if template.contains("{frame}") {
        template.replace("{frame}", &format!("{frame:06}"))
    } else {
        template.to_owned()
    }
}

pub fn resolve_runtime_config() -> Result<RuntimeConfig, String> {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    let (config_path, pass_through) = parse_config_path_arg(&raw_args)?;
    let runtime_args = parse_runtime_args(&pass_through)?;

    let loaded_config_path = config_path.clone().or_else(|| {
        let default_path = PathBuf::from(DEFAULT_CONFIG_PATH);
        if default_path.exists() {
            Some(default_path)
        } else {
            None
        }
    });
    let config = NesConfig::load_or_default(config_path.as_deref())?;

    let rom_path = runtime_args
        .rom_path
        .or_else(|| config.desktop.rom_path.clone())
        .or_else(|| config.roms.smb.clone())
        .ok_or_else(|| {
            format!(
                "ROM path not configured. Provide a positional ROM argument or set `desktop.rom_path`/`roms.smb` in {DEFAULT_CONFIG_PATH}."
            )
        })?;
    let window_scale = normalize_nonzero_u32(config.desktop.window_scale, DEFAULT_WINDOW_SCALE);
    let cpu_steps_per_frame = normalize_nonzero_u32(
        config.desktop.cpu_steps_per_frame,
        DEFAULT_CPU_STEPS_PER_FRAME,
    );
    let trace_every_frames = normalize_nonzero_u64(
        config.desktop.trace_every_frames,
        DEFAULT_TRACE_EVERY_FRAMES,
    );
    let metrics_every_frames = normalize_nonzero_u64(config.desktop.metrics_every_frames, 60);
    let capture = capture_config_from_parts(
        config.desktop.capture_path_template,
        config.desktop.capture_every_frames,
    );
    let netplay_enabled =
        netplay_feature_enabled(runtime_args.netplay_enabled, config.netplay.enabled);
    let step_mode = if netplay_enabled {
        StepMode::Frame
    } else {
        match config.desktop.step_mode {
            StepModeConfig::Frame => StepMode::Frame,
            StepModeConfig::Cpu => StepMode::CpuBudget(cpu_steps_per_frame),
        }
    };

    let netplay = if netplay_enabled {
        let relay_addr = runtime_args
            .netplay_relay_addr
            .or_else(|| Some(config.netplay.relay_addr.clone()))
            .unwrap_or_default();
        let room = runtime_args
            .netplay_room
            .or_else(|| Some(config.netplay.room.clone()))
            .unwrap_or_default();
        let player = runtime_args.netplay_player.unwrap_or(config.netplay.player);
        let input_delay_frames = runtime_args
            .netplay_input_delay_frames
            .unwrap_or(config.netplay.input_delay_frames);
        let max_rollback_frames = runtime_args
            .netplay_max_rollback_frames
            .unwrap_or(config.netplay.max_rollback_frames);
        let hash_check_every_frames = runtime_args
            .netplay_hash_check_every_frames
            .unwrap_or(config.netplay.hash_check_every_frames);
        if room.trim().is_empty() {
            return Err("netplay room cannot be empty".to_owned());
        }
        Some(NetplayRuntimeConfig {
            relay_addr,
            room,
            player,
            input_delay_frames,
            max_rollback_frames,
            hash_check_every_frames,
        })
    } else {
        None
    };
    let rta_enabled = runtime_args.rta_enabled
        || runtime_args.rta_profile_id.is_some()
        || runtime_args.rta_profiles_dir.is_some()
        || runtime_args.rta_runs_dir.is_some()
        || runtime_args.rta_calibrate;
    let rta = if rta_enabled {
        Some(RtaRuntimeConfig {
            profile_id_override: runtime_args.rta_profile_id.clone(),
            profiles_dir: PathBuf::from(
                runtime_args
                    .rta_profiles_dir
                    .clone()
                    .unwrap_or_else(|| DEFAULT_RTA_PROFILES_DIR.to_owned()),
            ),
            runs_dir: PathBuf::from(
                runtime_args
                    .rta_runs_dir
                    .clone()
                    .unwrap_or_else(|| DEFAULT_RTA_RUNS_DIR.to_owned()),
            ),
            calibrate: runtime_args.rta_calibrate,
        })
    } else {
        None
    };

    Ok(RuntimeConfig {
        rom_path,
        cheat_codes: runtime_args.cheat_codes,
        window_scale,
        step_mode,
        audio_enabled: config.desktop.audio_enabled,
        trace_every_frames,
        metrics_enabled: config.desktop.metrics_enabled,
        metrics_every_frames,
        capture,
        loaded_config_path,
        mcp_enabled: runtime_args.mcp_enabled,
        mcp_bind_addr: runtime_args.mcp_bind_addr,
        netplay,
        rta,
        #[cfg(feature = "nova")]
        auto_player_enabled: runtime_args.auto_player_enabled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_config_helpers_handle_placeholders_and_defaults() {
        assert!(capture_config_from_parts(None, 10).is_none());
        assert!(capture_config_from_parts(Some("   ".to_owned()), 10).is_none());

        let cfg = capture_config_from_parts(Some("snap-{frame}.ppm".to_owned()), 0)
            .expect("valid template should produce config");
        assert_eq!(cfg.path_template, "snap-{frame}.ppm");
        assert_eq!(cfg.every_n_frames, DEFAULT_CAPTURE_EVERY_FRAMES);

        assert_eq!(
            capture_path_for_frame("snap-{frame}.ppm", 42),
            "snap-000042.ppm"
        );
        assert_eq!(capture_path_for_frame("snap.ppm", 42), "snap.ppm");
    }

    #[test]
    fn test_netplay_feature_enabled() {
        assert!(netplay_feature_enabled(true, false));
        assert!(netplay_feature_enabled(false, true));
        assert!(!netplay_feature_enabled(false, false));
    }
}
