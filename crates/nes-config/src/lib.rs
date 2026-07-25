//! Configuration loading and deserialization for the NES emulator workspace.
//!
//! This crate parses `nes.toml` files and CLI overrides into a strongly-typed [`NesConfig`] struct.

use std::fs;
use std::path::{Path, PathBuf};

use crossterm::style::Stylize;
use serde::Deserialize;

/// Default path to the configuration file.
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

/// Execution granularity for the emulator loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StepModeConfig {
    /// Step the emulator one CPU instruction at a time.
    Cpu,
    /// Step the emulator one full PPU frame at a time.
    #[default]
    Frame,
}

/// Runtime configuration for desktop and windowing features.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DesktopConfig {
    /// Path to the ROM file to load on startup.
    pub rom_path: Option<String>,
    /// Multiplier for the native NES resolution.
    pub window_scale: u32,
    /// Emulator step granularity.
    pub step_mode: StepModeConfig,
    /// Fixed number of CPU steps to execute per frame when not syncing to PPU.
    pub cpu_steps_per_frame: u32,
    /// Toggles audio synthesis and playback.
    pub audio_enabled: bool,
    /// Interval (in frames) to emit CPU execution traces.
    pub trace_every_frames: u64,
    /// Toggles performance metrics collection.
    pub metrics_enabled: bool,
    /// Interval (in frames) to log metrics.
    pub metrics_every_frames: u64,
    /// Path template for saving audio captures.
    pub capture_path_template: Option<String>,
    /// Interval (in frames) for audio captures.
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

/// File paths to ROM files used for testing and validation.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RomPathsConfig {
    /// Path to Super Mario Bros ROM.
    pub smb: Option<String>,
    /// Path to nestest ROM.
    pub nestest: Option<String>,
    /// Path to blargg CPU test ROM.
    pub blargg_cpu: Option<String>,
    /// Directory containing bbbradsmith audio suite ROMs.
    pub bbbradsmith_audio_suite_dir: Option<String>,
    /// Directory containing bbbradsmith audio suite golden references.
    pub bbbradsmith_audio_golden_dir: Option<String>,
}

/// Settings for rollback netcode and online multiplayer.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetplayConfig {
    /// Toggles netplay rollback engine.
    pub enabled: bool,
    /// Address of the netplay relay server.
    pub relay_addr: String,
    /// Netplay room name.
    pub room: String,
    /// Player index (1 or 2).
    pub player: u8,
    /// Fixed input delay added before executing local inputs.
    pub input_delay_frames: u32,
    /// Maximum allowed rollback window size.
    pub max_rollback_frames: u32,
    /// Interval (in frames) to perform state hash checks.
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

/// Settings for the rewind and state history engine.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TimeMachineConfig {
    /// Toggles time machine functionality.
    pub enabled: bool,
    /// Maximum history size in seconds.
    pub max_history_seconds: u32,
    /// Frames between full state keyframes.
    pub keyframe_base_interval: u64,
    /// Size threshold in bytes to force a keyframe instead of a delta.
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

/// The root configuration object combining all sub-sections.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NesConfig {
    /// Desktop configuration.
    pub desktop: DesktopConfig,
    /// ROM paths configuration.
    pub roms: RomPathsConfig,
    /// Netplay configuration.
    pub netplay: NetplayConfig,
    /// Time machine configuration.
    pub time_machine: TimeMachineConfig,
}

impl NesConfig {
    /// Loads and parses a TOML configuration from the specified file path.
    ///
    /// # Examples
    /// ```no_run
    /// use nes_config::NesConfig;
    /// use std::path::Path;
    /// let config = NesConfig::load(Path::new("my_nes.toml")).unwrap();
    /// ```
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read_to_string(path).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                format!(
                    "{} Could not find the config file at '{}'.
{} copy the example profile (e.g. cp nes.example.toml nes.toml)",
                    "Error:".with(crossterm::style::Color::Red).bold(),
                    path.display()
                        .to_string()
                        .with(crossterm::style::Color::Yellow),
                    "Hint:".with(crossterm::style::Color::Cyan).bold()
                )
            } else {
                format!(
                    "{} Failed to read config at '{}': {err}",
                    "Error:".with(crossterm::style::Color::Red).bold(),
                    path.display()
                        .to_string()
                        .with(crossterm::style::Color::Yellow)
                )
            }
        })?;
        toml::from_str::<Self>(&bytes)
            .map_err(|err| format!("failed to parse config '{}': {err}", path.display()))
    }
    /// Loads a configuration from the given path if present, otherwise attempts to load from the default path or returns a default configuration.
    ///
    /// # Examples
    /// ```no_run
    /// use nes_config::NesConfig;
    /// let config = NesConfig::load_or_default(None).unwrap();
    /// ```
    pub fn load_or_default(path: Option<&Path>) -> Result<Self, String> {
        if let Some(config_path) = path {
            Self::load(config_path)
        } else {
            let default_path = Path::new(DEFAULT_CONFIG_PATH);
            if default_path.exists() {
                Self::load(default_path)
            } else {
                Ok(Self::default())
            }
        }
    }
}

/// Returns the value if it is non-zero, otherwise returns the fallback.
pub fn normalize_nonzero_u32(value: u32, fallback: u32) -> u32 {
    if value == 0 { fallback } else { value }
}

/// Returns the value if it is non-zero, otherwise returns the fallback.
pub fn normalize_nonzero_u64(value: u64, fallback: u64) -> u64 {
    if value == 0 { fallback } else { value }
}

/// Extracts the `--config` argument from a list of command-line arguments, returning the parsed path and the remaining arguments.
pub fn parse_config_path_arg(args: &[String]) -> Result<(Option<PathBuf>, Vec<String>), String> {
    let mut config_path = None::<PathBuf>;
    let mut pass_through = Vec::new();
    let mut idx = 0_usize;
    while idx < args.len() {
        let arg = &args[idx];
        if parse_arg(args, &mut idx, "--config", |value| {
            if value.starts_with("--") {
                return Err("missing value after --config".to_owned());
            }
            config_path = Some(PathBuf::from(value));
            Ok(())
        })? {
            continue;
        }
        pass_through.push(arg.clone());
        idx += 1;
    }
    Ok((config_path, pass_through))
}

fn parse_arg<F>(args: &[String], idx: &mut usize, flag: &str, mut apply: F) -> Result<bool, String>
where
    F: FnMut(&str) -> Result<(), String>,
{
    let arg = &args[*idx];
    if arg == flag {
        let Some(value) = args.get(*idx + 1) else {
            return Err(format!("missing value after {flag}"));
        };
        apply(value)?;
        *idx += 2;
        Ok(true)
    } else if let Some(value) = arg.strip_prefix(flag).and_then(|s| s.strip_prefix('=')) {
        if value.is_empty() {
            return Err(format!("missing value after {flag}="));
        }
        apply(value)?;
        *idx += 1;
        Ok(true)
    } else {
        Ok(false)
    }
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
        assert!(err.contains("Could not find the config file"));
        assert!(err.contains("copy the example profile"));
    }

    #[test]
    fn load_returns_underlying_error_for_non_not_found_io_errors() {
        let path = std::path::Path::new("."); // Reading a directory should cause IsADirectory or similar OS error
        let err = NesConfig::load(path).expect_err("loading a directory should fail");
        assert!(err.contains("Failed to read config"));
        assert!(!err.contains("copy the example profile"));
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
    fn parse_config_path_loop_terminates() {
        let args = vec![
            "--config".to_owned(),
            "dummy.toml".to_owned(),
            "extra".to_owned(),
        ];

        let start_idx = 0;
        let mut idx = start_idx;
        let res = super::parse_arg(&args, &mut idx, "--config", |_| Ok(()));
        assert_eq!(res, Ok(true));
        assert_eq!(idx, start_idx + 2); // Ensure it increments by 2

        let mut idx = 0;
        let args2 = vec!["--config=dummy.toml".to_owned(), "extra".to_owned()];
        let res2 = super::parse_arg(&args2, &mut idx, "--config", |_| Ok(()));
        assert_eq!(res2, Ok(true));
        assert_eq!(idx, 1); // Ensure it increments by 1

        let mut idx = 0;
        let args3 = vec!["extra".to_owned()];
        let res3 = super::parse_arg(&args3, &mut idx, "--config", |_| Ok(()));
        assert_eq!(res3, Ok(false));
        assert_eq!(idx, 0); // Unchanged when not matched
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
