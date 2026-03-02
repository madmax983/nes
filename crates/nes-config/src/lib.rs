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
#[serde(default)]
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
#[serde(default)]
pub struct RomPathsConfig {
    pub smb: Option<String>,
    pub nestest: Option<String>,
    pub blargg_cpu: Option<String>,
    pub bbbradsmith_audio_suite_dir: Option<String>,
    pub bbbradsmith_audio_golden_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct NesConfig {
    pub desktop: DesktopConfig,
    pub roms: RomPathsConfig,
    pub netplay: NetplayConfig,
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
    let mut idx = 0_usize;
    while idx < args.len() {
        let arg = &args[idx];
        if arg == "--config" {
            let Some(path) = args.get(idx + 1) else {
                return Err("missing value after --config".to_owned());
            };
            config_path = Some(PathBuf::from(path));
            idx += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--config=") {
            if !value.is_empty() {
                config_path = Some(PathBuf::from(value));
                idx += 1;
                continue;
            }
            return Err("missing value after --config=".to_owned());
        }
        pass_through.push(arg.clone());
        idx += 1;
    }
    Ok((config_path, pass_through))
}

#[cfg(test)]
mod tests {
    use super::parse_config_path_arg;

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
}
