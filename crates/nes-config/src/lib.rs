use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const DEFAULT_CONFIG_PATH: &str = "nes.toml";

const DEFAULT_CPU_STEPS_PER_FRAME: u32 = 10_000;
const DEFAULT_WINDOW_SCALE: u32 = 3;
const DEFAULT_AUDIO_ENABLED: bool = true;
const DEFAULT_METRICS_ENABLED: bool = false;
const DEFAULT_METRICS_EVERY_FRAMES: u64 = 60;
const DEFAULT_TRACE_EVERY_FRAMES: u64 = 0;
const DEFAULT_CAPTURE_EVERY_FRAMES: u64 = 1;
const DEFAULT_NETPLAY_ENABLED: bool = false;
const DEFAULT_NETPLAY_RELAY_ADDR: &str = "127.0.0.1:4545";
const DEFAULT_NETPLAY_ROOM: &str = "default";
const DEFAULT_NETPLAY_PLAYER: u8 = 1;
const DEFAULT_NETPLAY_INPUT_DELAY_FRAMES: u32 = 2;
const DEFAULT_NETPLAY_MAX_ROLLBACK_FRAMES: u32 = 240;
const DEFAULT_NETPLAY_HASH_CHECK_EVERY_FRAMES: u64 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StepModeConfig {
    Cpu,
    #[default]
    Frame,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DesktopConfig {
    pub rom_path: Option<String>,
    pub window_scale: u32,
    pub step_mode: StepModeConfig,
    pub cpu_steps_per_frame: u32,
    pub audio_enabled: bool,
    pub trace_every_frames: u64,
    pub metrics_enabled: bool,
    pub metrics_every_frames: u64,
    pub capture_path_template: Option<String>,
    pub capture_every_frames: u64,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            rom_path: None,
            window_scale: DEFAULT_WINDOW_SCALE,
            step_mode: StepModeConfig::Frame,
            cpu_steps_per_frame: DEFAULT_CPU_STEPS_PER_FRAME,
            audio_enabled: DEFAULT_AUDIO_ENABLED,
            trace_every_frames: DEFAULT_TRACE_EVERY_FRAMES,
            metrics_enabled: DEFAULT_METRICS_ENABLED,
            metrics_every_frames: DEFAULT_METRICS_EVERY_FRAMES,
            capture_path_template: None,
            capture_every_frames: DEFAULT_CAPTURE_EVERY_FRAMES,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RomPathsConfig {
    pub smb: Option<String>,
    pub nestest: Option<String>,
    pub blargg_cpu: Option<String>,
    pub bbbradsmith_audio_suite_dir: Option<String>,
    pub bbbradsmith_audio_golden_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetplayConfig {
    pub enabled: bool,
    pub relay_addr: String,
    pub room: String,
    pub player: u8,
    pub input_delay_frames: u32,
    pub max_rollback_frames: u32,
    pub hash_check_every_frames: u64,
}

impl Default for NetplayConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_NETPLAY_ENABLED,
            relay_addr: DEFAULT_NETPLAY_RELAY_ADDR.to_owned(),
            room: DEFAULT_NETPLAY_ROOM.to_owned(),
            player: DEFAULT_NETPLAY_PLAYER,
            input_delay_frames: DEFAULT_NETPLAY_INPUT_DELAY_FRAMES,
            max_rollback_frames: DEFAULT_NETPLAY_MAX_ROLLBACK_FRAMES,
            hash_check_every_frames: DEFAULT_NETPLAY_HASH_CHECK_EVERY_FRAMES,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TimeMachineConfig {
    pub enabled: bool,
    pub max_history_seconds: u32,
    pub keyframe_base_interval: u64,
    pub delta_spike_threshold: u32,
}

impl Default for TimeMachineConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_history_seconds: 30,
            keyframe_base_interval: 60,
            delta_spike_threshold: 2048,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NesConfig {
    pub desktop: DesktopConfig,
    pub roms: RomPathsConfig,
    pub netplay: NetplayConfig,
    pub time_machine: TimeMachineConfig,
}

impl NesConfig {
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read_to_string(path)
            .map_err(|err| format!("failed to read config '{}': {err}", path.display()))?;
        toml::from_str::<Self>(&bytes)
            .map_err(|err| format!("failed to parse config '{}': {err}", path.display()))
    }

    pub fn load_or_default(path: Option<&Path>) -> Result<Self, String> {
        match path {
            Some(config_path) => Self::load(config_path),
            None => {
                let default_path = Path::new(DEFAULT_CONFIG_PATH);
                if default_path.exists() {
                    Self::load(default_path)
                } else {
                    Ok(Self::default())
                }
            }
        }
    }
}

pub fn normalize_nonzero_u32(value: u32, fallback: u32) -> u32 {
    if value == 0 { fallback } else { value }
}

pub fn normalize_nonzero_u64(value: u64, fallback: u64) -> u64 {
    if value == 0 { fallback } else { value }
}

pub fn parse_config_path_arg(args: &[String]) -> Result<(Option<PathBuf>, Vec<String>), String> {
    let mut config_path = None::<PathBuf>;
    let mut pass_through = Vec::new();
    let mut arg_iter = args.iter();
    while let Some(arg) = arg_iter.next() {
        if arg == "--config" {
            let Some(path) = arg_iter.next() else {
                return Err("missing value after --config".to_owned());
            };
            if path.starts_with("--") {
                return Err("missing value after --config".to_owned());
            }
            config_path = Some(PathBuf::from(path));
            continue;
        }
        if let Some(value) = arg.strip_prefix("--config=") {
            if !value.is_empty() {
                config_path = Some(PathBuf::from(value));
                continue;
            }
            return Err("missing value after --config=".to_owned());
        }
        pass_through.push(arg.clone());
    }
    Ok((config_path, pass_through))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::{NesConfig, normalize_nonzero_u32, normalize_nonzero_u64, parse_config_path_arg};

    fn temp_config_path(stem: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("nes-config-{stem}-{nonce}.toml"))
    }

    fn write_temp_config(contents: &str, stem: &str) -> PathBuf {
        let path = temp_config_path(stem);
        fs::write(&path, contents).expect("should write temp config");
        path
    }

    fn remove_if_exists(path: &PathBuf) {
        if path.exists() {
            fs::remove_file(path).expect("should remove temp file");
        }
    }

    #[test]
    fn time_machine_config_has_sane_defaults() {
        let config = NesConfig::default();
        assert!(config.time_machine.enabled);
        assert_eq!(config.time_machine.max_history_seconds, 30);
        assert_eq!(config.time_machine.keyframe_base_interval, 60);
        assert_eq!(config.time_machine.delta_spike_threshold, 2048);
    }

    #[test]
    fn load_reads_non_default_values_from_file() {
        let path = write_temp_config(
            r#"
[desktop]
window_scale = 7
audio_enabled = false

[netplay]
enabled = true
room = "arena"
"#,
            "load-non-defaults",
        );
        let loaded = NesConfig::load(&path).expect("load should succeed");
        remove_if_exists(&path);

        assert_eq!(loaded.desktop.window_scale, 7);
        assert!(!loaded.desktop.audio_enabled);
        assert!(loaded.netplay.enabled);
        assert_eq!(loaded.netplay.room, "arena");
    }

    #[test]
    fn load_returns_error_for_missing_file() {
        let path = temp_config_path("missing");
        let err = NesConfig::load(&path).expect_err("missing config should fail");
        assert!(err.contains("failed to read config"));
    }

    #[test]
    fn load_or_default_uses_provided_path_when_present() {
        let path = write_temp_config(
            r#"
[desktop]
window_scale = 9
"#,
            "load-or-default-path",
        );
        let loaded = NesConfig::load_or_default(Some(&path)).expect("load_or_default should load");
        remove_if_exists(&path);

        assert_eq!(loaded.desktop.window_scale, 9);
    }

    #[test]
    fn normalize_nonzero_helpers_use_fallback_only_for_zero() {
        assert_eq!(normalize_nonzero_u32(0, 77), 77);
        assert_eq!(normalize_nonzero_u32(5, 77), 5);
        assert_eq!(normalize_nonzero_u64(0, 99), 99);
        assert_eq!(normalize_nonzero_u64(42, 99), 42);
    }

    #[test]
    fn parse_config_path_arg_supports_split_flag() {
        let args = vec![
            "--config".to_owned(),
            "custom.toml".to_owned(),
            "rom.nes".to_owned(),
        ];
        let (config, rest) = parse_config_path_arg(&args).expect("parse should succeed");
        assert_eq!(
            config.as_deref().and_then(|p| p.to_str()),
            Some("custom.toml")
        );
        assert_eq!(rest, vec!["rom.nes".to_owned()]);
    }

    #[test]
    fn parse_config_path_arg_supports_equals_flag() {
        let args = vec!["--config=custom.toml".to_owned(), "rom.nes".to_owned()];
        let (config, rest) = parse_config_path_arg(&args).expect("parse should succeed");
        assert_eq!(
            config.as_deref().and_then(|p| p.to_str()),
            Some("custom.toml")
        );
        assert_eq!(rest, vec!["rom.nes".to_owned()]);
    }

    #[test]
    fn parse_config_path_arg_rejects_missing_value() {
        let args = vec!["--config".to_owned()];
        let err = parse_config_path_arg(&args).expect_err("parse should fail");
        assert!(err.contains("missing value"));
    }

    #[test]
    fn parse_config_path_arg_rejects_flag_as_split_value() {
        let args = vec!["--config".to_owned(), "--netplay".to_owned()];
        let err = parse_config_path_arg(&args).expect_err("parse should fail");
        assert!(err.contains("missing value"));
    }

    #[test]
    fn parse_config_path_arg_rejects_empty_equals_value() {
        let args = vec!["--config=".to_owned()];
        let err = parse_config_path_arg(&args).expect_err("parse should fail");
        assert!(err.contains("missing value"));
    }

    #[test]
    fn parse_config_path_arg_last_flag_wins_and_preserves_other_args() {
        let args = vec![
            "--config=first.toml".to_owned(),
            "rom.nes".to_owned(),
            "--config".to_owned(),
            "second.toml".to_owned(),
            "--fullscreen".to_owned(),
        ];
        let (config, rest) = parse_config_path_arg(&args).expect("parse should succeed");
        assert_eq!(
            config.as_deref(),
            Some(PathBuf::from("second.toml").as_path())
        );
        assert_eq!(rest, vec!["rom.nes".to_owned(), "--fullscreen".to_owned()]);
    }

    #[test]
    fn load_rejects_unknown_top_level_field() {
        let path = write_temp_config(
            r#"
unknown_top_level = true
"#,
            "load-unknown-top-level",
        );
        let err = NesConfig::load(&path).expect_err("unknown top-level field should fail");
        remove_if_exists(&path);

        assert!(err.contains("unknown field"));
        assert!(err.contains("unknown_top_level"));
    }

    #[test]
    fn load_rejects_unknown_nested_field() {
        let path = write_temp_config(
            r#"
[desktop]
window_scale = 4
window_scal = 7
"#,
            "load-unknown-nested",
        );
        let err = NesConfig::load(&path).expect_err("unknown nested field should fail");
        remove_if_exists(&path);

        assert!(err.contains("unknown field"));
        assert!(err.contains("window_scal"));
    }
}
