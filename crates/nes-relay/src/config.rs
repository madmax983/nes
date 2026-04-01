#[derive(Debug, Clone, Copy, Default)]
pub struct LinkCondition {
    pub latency_ms: u64,
    pub jitter_ms: u64,
    pub loss_pct: u8,
    pub reorder_pct: u8,
}

#[derive(Debug, Clone)]
pub struct RelayArgs {
    pub bind_addr: String,
    pub link: LinkCondition,
}

/// Parses raw command-line arguments into a structured `RelayArgs` configuration.
///
/// This function acts as the entry point for the `nes-relay` configuration parsing. It maps
/// command-line flags (e.g., `--bind`, `--latency-ms`) to their respective fields in the
/// [`RelayArgs`] struct. Unrecognized arguments or malformed values will return an error string.
///
/// ## Examples
///
/// ```rust
/// use nes_relay::config::parse_args;
///
/// let args = vec!["--latency-ms".to_string(), "50".to_string()];
/// let parsed = parse_args(args).unwrap();
///
/// assert_eq!(parsed.link.latency_ms, 50);
/// ```
pub fn parse_args(args: Vec<String>) -> Result<RelayArgs, String> {
    let mut parsed = RelayArgs {
        bind_addr: "127.0.0.1:4545".to_owned(),
        link: LinkCondition::default(),
    };
    let mut idx = 0_usize;
    while idx < args.len() {
        let arg = &args[idx];
        if arg == "--help" || arg == "-h" {
            return Err("Usage: nes-relay [--bind <addr>] [--latency-ms <n>] [--jitter-ms <n>] [--loss-pct <0..100>] [--reorder-pct <0..100>]\nDefault bind: 127.0.0.1:4545".to_string());
        }

        if parse_arg(&args, &mut idx, "--bind", |value| {
            parsed.bind_addr = value.to_string();
            Ok(())
        })? {
            continue;
        }

        if parse_arg(&args, &mut idx, "--latency-ms", |value| {
            parsed.link.latency_ms = parse_u64_arg(value, "--latency-ms")?;
            Ok(())
        })? {
            continue;
        }

        if parse_arg(&args, &mut idx, "--jitter-ms", |value| {
            parsed.link.jitter_ms = parse_u64_arg(value, "--jitter-ms")?;
            Ok(())
        })? {
            continue;
        }

        if parse_arg(&args, &mut idx, "--loss-pct", |value| {
            parsed.link.loss_pct = parse_percent_arg(value, "--loss-pct")?;
            Ok(())
        })? {
            continue;
        }

        if parse_arg(&args, &mut idx, "--reorder-pct", |value| {
            parsed.link.reorder_pct = parse_percent_arg(value, "--reorder-pct")?;
            Ok(())
        })? {
            continue;
        }

        return Err(format!(
            "unknown argument '{arg}'. Usage: nes-relay [--bind <addr>] [--latency-ms <n>] [--jitter-ms <n>] [--loss-pct <0..100>] [--reorder-pct <0..100>]"
        ));
    }
    Ok(parsed)
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

/// Parses a string representation of a non-negative integer into a `u64`.
///
/// This utility function is used to parse configuration values like latency or jitter
/// that must be purely numeric. If the parse fails, it returns a descriptive error mentioning
/// the original `flag` that failed.
///
/// ## Examples
///
/// ```rust
/// use nes_relay::config::parse_u64_arg;
///
/// assert_eq!(parse_u64_arg("42", "--latency-ms"), Ok(42));
/// assert!(parse_u64_arg("abc", "--latency-ms").is_err());
/// ```
pub fn parse_u64_arg(value: &str, flag: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{flag} must be a non-negative integer"))
}

/// Parses a string representation of a percentage into a `u8` bounded between 0 and 100.
///
/// This function is specifically used for parsing network loss and reordering probabilities.
/// It ensures that users cannot configure impossible conditions (like 150% packet loss).
///
/// ## Examples
///
/// ```rust
/// use nes_relay::config::parse_percent_arg;
///
/// assert_eq!(parse_percent_arg("50", "--loss-pct"), Ok(50));
/// assert!(parse_percent_arg("101", "--loss-pct").is_err());
/// assert!(parse_percent_arg("-5", "--loss-pct").is_err());
/// ```
pub fn parse_percent_arg(value: &str, flag: &str) -> Result<u8, String> {
    let parsed = value
        .parse::<u8>()
        .map_err(|_| format!("{flag} must be an integer in [0, 100]"))?;
    if parsed > 100 {
        return Err(format!("{flag} must be in [0, 100]"));
    }
    Ok(parsed)
}
