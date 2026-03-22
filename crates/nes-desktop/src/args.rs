use nes_config::DEFAULT_CONFIG_PATH;

pub const DEFAULT_MCP_BIND_ADDR: &str = "127.0.0.1:6502";
pub const RUNTIME_USAGE: &str = "Usage: nes-desktop [--config <path>] [--cheat-code <code>] [--mcp-host] [--mcp-bind <addr>] [--netplay] [--netplay-relay <addr>] [--netplay-room <room>] [--netplay-player <1|2>] [--netplay-delay <frames>] [--netplay-max-rollback <frames>] [--netplay-hash-every <frames>] [--rta] [--rta-profile <id>] [--rta-profiles-dir <path>] [--rta-runs-dir <path>] [--rta-calibrate] [rom_path]";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeArgs {
    pub rom_path: Option<String>,
    pub cheat_codes: Vec<String>,
    pub mcp_enabled: bool,
    pub mcp_bind_addr: String,
    pub netplay_enabled: bool,
    pub netplay_relay_addr: Option<String>,
    pub netplay_room: Option<String>,
    pub netplay_player: Option<u8>,
    pub netplay_input_delay_frames: Option<u32>,
    pub netplay_max_rollback_frames: Option<u32>,
    pub netplay_hash_check_every_frames: Option<u64>,
    pub rta_enabled: bool,
    pub rta_profile_id: Option<String>,
    pub rta_profiles_dir: Option<String>,
    pub rta_runs_dir: Option<String>,
    pub rta_calibrate: bool,
    #[cfg(feature = "nova")]
    pub auto_player_enabled: bool,
}

/// Parses command-line arguments into the typed `RuntimeArgs` configuration.
///
/// This handles routing for CLI flags that dictate whether the emulator should launch
/// with MCP, Netplay, or RTA modes enabled.
///
/// ## Examples
///
/// ```
/// use nes_desktop::args::parse_runtime_args;
///
/// let args = vec![
///     "--mcp-host".to_string(),
///     "./game.nes".to_string()
/// ];
/// let parsed = parse_runtime_args(&args).unwrap();
/// assert!(parsed.mcp_enabled);
/// assert_eq!(parsed.rom_path.as_deref(), Some("./game.nes"));
/// ```
pub fn parse_runtime_args(args: &[String]) -> Result<RuntimeArgs, String> {
    let mut parsed = RuntimeArgs {
        rom_path: None,
        cheat_codes: Vec::new(),
        mcp_enabled: false,
        mcp_bind_addr: DEFAULT_MCP_BIND_ADDR.to_owned(),
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
        #[cfg(feature = "nova")]
        auto_player_enabled: false,
    };
    let mut idx = 0_usize;
    while idx < args.len() {
        let arg = &args[idx];
        if arg == "--help" || arg == "-h" {
            return Err(format!(
                "{RUNTIME_USAGE}\nDefault config path: {DEFAULT_CONFIG_PATH}"
            ));
        }
        if arg == "--cheat-code" {
            let Some(code) = args.get(idx + 1) else {
                return Err("missing value after --cheat-code".to_owned());
            };
            parsed.cheat_codes.push(code.clone());
            idx += 2;
            continue;
        }
        if let Some(code) = arg.strip_prefix("--cheat-code=") {
            parsed.cheat_codes.push(code.to_owned());
            idx += 1;
            continue;
        }
        if arg == "--mcp-host" {
            parsed.mcp_enabled = true;
            idx += 1;
            continue;
        }
        if arg == "--mcp-bind" {
            let Some(addr) = args.get(idx + 1) else {
                return Err("missing value after --mcp-bind".to_owned());
            };
            parsed.mcp_enabled = true;
            parsed.mcp_bind_addr = addr.clone();
            idx += 2;
            continue;
        }
        if let Some(addr) = arg.strip_prefix("--mcp-bind=") {
            parsed.mcp_enabled = true;
            parsed.mcp_bind_addr = addr.to_owned();
            idx += 1;
            continue;
        }
        if arg == "--netplay" {
            parsed.netplay_enabled = true;
            idx += 1;
            continue;
        }
        if arg == "--netplay-relay" {
            let Some(addr) = args.get(idx + 1) else {
                return Err("missing value after --netplay-relay".to_owned());
            };
            parsed.netplay_enabled = true;
            parsed.netplay_relay_addr = Some(addr.clone());
            idx += 2;
            continue;
        }
        if let Some(addr) = arg.strip_prefix("--netplay-relay=") {
            parsed.netplay_enabled = true;
            parsed.netplay_relay_addr = Some(addr.to_owned());
            idx += 1;
            continue;
        }
        if arg == "--netplay-room" {
            let Some(room) = args.get(idx + 1) else {
                return Err("missing value after --netplay-room".to_owned());
            };
            parsed.netplay_enabled = true;
            parsed.netplay_room = Some(room.clone());
            idx += 2;
            continue;
        }
        if let Some(room) = arg.strip_prefix("--netplay-room=") {
            parsed.netplay_enabled = true;
            parsed.netplay_room = Some(room.to_owned());
            idx += 1;
            continue;
        }
        if arg == "--netplay-player" {
            let Some(val) = args.get(idx + 1) else {
                return Err("missing value after --netplay-player".to_owned());
            };
            parsed.netplay_enabled = true;
            parsed.netplay_player = Some(parse_u8_arg(val, "--netplay-player")?);
            idx += 2;
            continue;
        }
        if let Some(val) = arg.strip_prefix("--netplay-player=") {
            parsed.netplay_enabled = true;
            parsed.netplay_player = Some(parse_u8_arg(val, "--netplay-player")?);
            idx += 1;
            continue;
        }
        if arg == "--netplay-delay" {
            let Some(val) = args.get(idx + 1) else {
                return Err("missing value after --netplay-delay".to_owned());
            };
            parsed.netplay_enabled = true;
            parsed.netplay_input_delay_frames = Some(parse_u32_arg(val, "--netplay-delay")?);
            idx += 2;
            continue;
        }
        if let Some(val) = arg.strip_prefix("--netplay-delay=") {
            parsed.netplay_enabled = true;
            parsed.netplay_input_delay_frames = Some(parse_u32_arg(val, "--netplay-delay")?);
            idx += 1;
            continue;
        }
        if arg == "--netplay-max-rollback" {
            let Some(val) = args.get(idx + 1) else {
                return Err("missing value after --netplay-max-rollback".to_owned());
            };
            parsed.netplay_enabled = true;
            parsed.netplay_max_rollback_frames =
                Some(parse_u32_arg(val, "--netplay-max-rollback")?);
            idx += 2;
            continue;
        }
        if let Some(val) = arg.strip_prefix("--netplay-max-rollback=") {
            parsed.netplay_enabled = true;
            parsed.netplay_max_rollback_frames =
                Some(parse_u32_arg(val, "--netplay-max-rollback")?);
            idx += 1;
            continue;
        }
        if arg == "--netplay-hash-every" {
            let Some(val) = args.get(idx + 1) else {
                return Err("missing value after --netplay-hash-every".to_owned());
            };
            parsed.netplay_enabled = true;
            parsed.netplay_hash_check_every_frames =
                Some(parse_u64_arg(val, "--netplay-hash-every")?);
            idx += 2;
            continue;
        }
        if let Some(val) = arg.strip_prefix("--netplay-hash-every=") {
            parsed.netplay_enabled = true;
            parsed.netplay_hash_check_every_frames =
                Some(parse_u64_arg(val, "--netplay-hash-every")?);
            idx += 1;
            continue;
        }
        if arg == "--rta" {
            parsed.rta_enabled = true;
            idx += 1;
            continue;
        }
        if arg == "--rta-profile" {
            let Some(id) = args.get(idx + 1) else {
                return Err("missing value after --rta-profile".to_owned());
            };
            parsed.rta_enabled = true;
            parsed.rta_profile_id = Some(id.clone());
            idx += 2;
            continue;
        }
        if let Some(id) = arg.strip_prefix("--rta-profile=") {
            parsed.rta_enabled = true;
            parsed.rta_profile_id = Some(id.to_owned());
            idx += 1;
            continue;
        }
        if arg == "--rta-profiles-dir" {
            let Some(dir) = args.get(idx + 1) else {
                return Err("missing value after --rta-profiles-dir".to_owned());
            };
            parsed.rta_enabled = true;
            parsed.rta_profiles_dir = Some(dir.clone());
            idx += 2;
            continue;
        }
        if let Some(dir) = arg.strip_prefix("--rta-profiles-dir=") {
            parsed.rta_enabled = true;
            parsed.rta_profiles_dir = Some(dir.to_owned());
            idx += 1;
            continue;
        }
        if arg == "--rta-runs-dir" {
            let Some(dir) = args.get(idx + 1) else {
                return Err("missing value after --rta-runs-dir".to_owned());
            };
            parsed.rta_enabled = true;
            parsed.rta_runs_dir = Some(dir.clone());
            idx += 2;
            continue;
        }
        if let Some(dir) = arg.strip_prefix("--rta-runs-dir=") {
            parsed.rta_enabled = true;
            parsed.rta_runs_dir = Some(dir.to_owned());
            idx += 1;
            continue;
        }
        if arg == "--rta-calibrate" {
            parsed.rta_enabled = true;
            parsed.rta_calibrate = true;
            idx += 1;
            continue;
        }
        #[cfg(feature = "nova")]
        if arg == "--auto-player" {
            parsed.auto_player_enabled = true;
            idx += 1;
            continue;
        }
        if arg.starts_with("--") {
            return Err(format!("unknown flag: {arg}"));
        }
        if let Some(path) = &parsed.rom_path {
            return Err(format!("multiple ROM paths provided: {} and {}", path, arg));
        }
        parsed.rom_path = Some(arg.clone());
        idx += 1;
    }
    Ok(parsed)
}

fn parse_u8_arg(value: &str, flag: &str) -> Result<u8, String> {
    value
        .parse::<u8>()
        .map_err(|_| format!("{flag} must be an integer in [0, 255]"))
}

fn parse_u32_arg(value: &str, flag: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("{flag} must be a non-negative integer"))
}

fn parse_u64_arg(value: &str, flag: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{flag} must be a non-negative integer"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    fn parse_runtime_args_with_timeout(args: Vec<String>) -> Result<RuntimeArgs, String> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = parse_runtime_args(&args);
            let _ = tx.send(result);
        });
        rx.recv_timeout(Duration::from_millis(100))
            .expect("parse_runtime_args blocked or panicked")
    }

    #[test]
    fn parse_runtime_args_accepts_mcp_host_and_bind_flags() {
        let args = vec![
            "--mcp-host".to_owned(),
            "--mcp-bind".to_owned(),
            "127.0.0.1:9999".to_owned(),
            "game.nes".to_owned(),
        ];
        let parsed = parse_runtime_args_with_timeout(args).expect("parse args");
        assert_eq!(parsed.rom_path.as_deref(), Some("game.nes"));
        assert!(parsed.cheat_codes.is_empty());
        assert!(parsed.mcp_enabled);
        assert_eq!(parsed.mcp_bind_addr, "127.0.0.1:9999");
    }

    #[test]
    fn parse_runtime_args_accepts_equals_bind_form() {
        let args = vec![
            "--mcp-host".to_owned(),
            "--mcp-bind=0.0.0.0:8888".to_owned(),
            "game.nes".to_owned(),
        ];
        let parsed = parse_runtime_args_with_timeout(args).expect("parse args");
        assert_eq!(parsed.mcp_bind_addr, "0.0.0.0:8888");
    }

    #[test]
    fn parse_runtime_args_defaults_bind_when_flag_absent() {
        let args = vec!["game.nes".to_owned()];
        let parsed = parse_runtime_args_with_timeout(args).expect("parse args");
        assert_eq!(parsed.rom_path.as_deref(), Some("game.nes"));
        assert!(parsed.cheat_codes.is_empty());
        assert!(!parsed.mcp_enabled);
        assert_eq!(parsed.mcp_bind_addr, DEFAULT_MCP_BIND_ADDR);
    }

    #[test]
    fn parse_runtime_args_rejects_unknown_flags() {
        let args = vec!["--bogus".to_owned()];
        let err = parse_runtime_args_with_timeout(args).expect_err("unknown flag should fail");
        assert!(err.contains("unknown flag"));
    }

    #[test]
    fn parse_runtime_args_accepts_netplay_flags() {
        let args = vec![
            "--netplay".to_owned(),
            "--netplay-relay=relay.example:4545".to_owned(),
            "--netplay-room".to_owned(),
            "river-city".to_owned(),
            "--netplay-player".to_owned(),
            "2".to_owned(),
            "--netplay-delay=3".to_owned(),
            "--netplay-max-rollback".to_owned(),
            "360".to_owned(),
            "--netplay-hash-every".to_owned(),
            "90".to_owned(),
        ];
        let parsed = parse_runtime_args_with_timeout(args).expect("parse args");
        let expected = RuntimeArgs {
            rom_path: None,
            cheat_codes: Vec::new(),
            mcp_enabled: false,
            mcp_bind_addr: DEFAULT_MCP_BIND_ADDR.to_owned(),
            netplay_enabled: true,
            netplay_relay_addr: Some("relay.example:4545".to_owned()),
            netplay_room: Some("river-city".to_owned()),
            netplay_player: Some(2),
            netplay_input_delay_frames: Some(3),
            netplay_max_rollback_frames: Some(360),
            netplay_hash_check_every_frames: Some(90),
            rta_enabled: false,
            rta_profile_id: None,
            rta_profiles_dir: None,
            rta_runs_dir: None,
            rta_calibrate: false,
            #[cfg(feature = "nova")]
            auto_player_enabled: false,
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_runtime_args_accepts_repeated_cheat_code_flags() {
        let args = vec![
            "--cheat-code".to_owned(),
            "gossip".to_owned(),
            "--cheat-code=ZEXPYGLA".to_owned(),
            "rom.nes".to_owned(),
        ];
        let parsed = parse_runtime_args_with_timeout(args).expect("cheat code args should parse");

        assert_eq!(parsed.rom_path.as_deref(), Some("rom.nes"));
        assert_eq!(
            parsed.cheat_codes,
            vec!["gossip".to_owned(), "ZEXPYGLA".to_owned()]
        );
    }

    #[test]
    fn parse_runtime_args_accepts_rta_flags() {
        let args = vec![
            "--rta".to_owned(),
            "--rta-profile=smb-any".to_owned(),
            "--rta-profiles-dir".to_owned(),
            "config/rta/profiles".to_owned(),
            "--rta-runs-dir=runs/rta".to_owned(),
            "--rta-calibrate".to_owned(),
        ];
        let parsed = parse_runtime_args_with_timeout(args).expect("parse args");

        assert!(parsed.rta_enabled);
        assert_eq!(parsed.rta_profile_id.as_deref(), Some("smb-any"));
        assert_eq!(
            parsed.rta_profiles_dir.as_deref(),
            Some("config/rta/profiles")
        );
        assert_eq!(parsed.rta_runs_dir.as_deref(), Some("runs/rta"));
        assert!(parsed.rta_calibrate);
    }

    #[test]
    fn parse_runtime_args_help_and_validation_paths() {
        let help = parse_runtime_args_with_timeout(vec!["--help".to_owned()])
            .expect_err("help returns usage");
        assert!(help.contains("Usage: nes-desktop"));
        assert!(help.contains("Default config path"));
        assert!(help.contains("--cheat-code <code>"));

        let missing_bind = parse_runtime_args_with_timeout(vec!["--mcp-bind".to_owned()])
            .expect_err("missing mcp bind");
        assert!(missing_bind.contains("missing value after --mcp-bind"));

        let missing_code = parse_runtime_args_with_timeout(vec!["--cheat-code".to_owned()])
            .expect_err("missing cheat code");
        assert!(missing_code.contains("missing value after --cheat-code"));

        let missing_relay = parse_runtime_args_with_timeout(vec!["--netplay-relay".to_owned()])
            .expect_err("missing netplay relay");
        assert!(missing_relay.contains("missing value after --netplay-relay"));

        let invalid_player =
            parse_runtime_args_with_timeout(vec!["--netplay-player".to_owned(), "bad".to_owned()])
                .expect_err("invalid player value");
        assert!(invalid_player.contains("--netplay-player"));

        let multi_rom =
            parse_runtime_args_with_timeout(vec!["a.nes".to_owned(), "b.nes".to_owned()])
                .expect_err("multiple roms should fail");
        assert!(multi_rom.contains("multiple ROM paths"));
    }

    #[test]
    fn parse_runtime_args_accepts_all_equals_forms_for_netplay_flags() {
        let args = vec![
            "--mcp-host".to_owned(),
            "--mcp-bind=127.0.0.1:7777".to_owned(),
            "--netplay".to_owned(),
            "--netplay-relay=relay.example:4545".to_owned(),
            "--netplay-room=arena".to_owned(),
            "--netplay-player=2".to_owned(),
            "--netplay-delay=3".to_owned(),
            "--netplay-max-rollback=360".to_owned(),
            "--netplay-hash-every=90".to_owned(),
            "rom.nes".to_owned(),
        ];
        let parsed = parse_runtime_args_with_timeout(args).expect("equals forms should parse");
        assert_eq!(parsed.rom_path.as_deref(), Some("rom.nes"));
        assert!(parsed.mcp_enabled);
        assert_eq!(parsed.mcp_bind_addr, "127.0.0.1:7777");
        assert!(parsed.netplay_enabled);
        assert_eq!(
            parsed.netplay_relay_addr.as_deref(),
            Some("relay.example:4545")
        );
        assert_eq!(parsed.netplay_room.as_deref(), Some("arena"));
        assert_eq!(parsed.netplay_player, Some(2));
        assert_eq!(parsed.netplay_input_delay_frames, Some(3));
        assert_eq!(parsed.netplay_max_rollback_frames, Some(360));
        assert_eq!(parsed.netplay_hash_check_every_frames, Some(90));
    }

    #[test]
    fn parse_runtime_args_control_flow_branches_do_not_hang() {
        let mcp_host = parse_runtime_args_with_timeout(vec!["--mcp-host".to_owned()])
            .expect("mcp host branch should parse");
        assert!(mcp_host.mcp_enabled);

        let netplay = parse_runtime_args_with_timeout(vec!["--netplay".to_owned()])
            .expect("netplay branch should parse");
        assert!(netplay.netplay_enabled);

        let relay = parse_runtime_args_with_timeout(vec![
            "--netplay-relay".to_owned(),
            "relay.example:4545".to_owned(),
        ])
        .expect("netplay relay branch should parse");
        assert_eq!(
            relay.netplay_relay_addr.as_deref(),
            Some("relay.example:4545")
        );

        let room =
            parse_runtime_args_with_timeout(vec!["--netplay-room".to_owned(), "arena".to_owned()])
                .expect("netplay room branch should parse");
        assert_eq!(room.netplay_room.as_deref(), Some("arena"));

        let player =
            parse_runtime_args_with_timeout(vec!["--netplay-player".to_owned(), "2".to_owned()])
                .expect("netplay player branch should parse");
        assert_eq!(player.netplay_player, Some(2));

        let delay =
            parse_runtime_args_with_timeout(vec!["--netplay-delay".to_owned(), "3".to_owned()])
                .expect("netplay delay branch should parse");
        assert_eq!(delay.netplay_input_delay_frames, Some(3));

        let hash_every = parse_runtime_args_with_timeout(vec![
            "--netplay-hash-every".to_owned(),
            "90".to_owned(),
        ])
        .expect("netplay hash-every branch should parse");
        assert_eq!(hash_every.netplay_hash_check_every_frames, Some(90));
    }
}
