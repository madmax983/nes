/// Configures simulated network degradation for testing netcode resilience.
///
/// This struct allows you to inject artificial latency, jitter, packet loss, and reordering
/// to simulate poor real-world internet conditions.
///
/// A netcode system is only as robust as the chaos it can endure. `LinkCondition` transforms
/// a pristine local loopback into a stormy ocean of dropped packets and wildly fluctuating latency.
///
/// ## Examples
///
/// ```rust
/// use nes_relay::config::LinkCondition;
///
/// let condition = LinkCondition {
///     latency_ms: 100,
///     jitter_ms: 20,
///     loss_pct: 5,
///     reorder_pct: 1,
/// };
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct LinkCondition {
    /// The baseline artificial latency to inject (in milliseconds).
    pub latency_ms: u64,
    /// The maximum variance in latency (in milliseconds).
    pub jitter_ms: u64,
    /// The percentage probability (0-100) of dropping a packet.
    pub loss_pct: u8,
    /// The percentage probability (0-100) of reordering a packet.
    pub reorder_pct: u8,
}

/// The parsed configuration arguments for the relay server.
///
/// This encapsulates both the physical binding instructions for the server and the
/// simulated atmospheric conditions (the `LinkCondition`) it should impose upon all
/// who connect.
///
/// ## Examples
///
/// ```rust
/// use nes_relay::config::{RelayArgs, LinkCondition};
///
/// let args = RelayArgs {
///     bind_addr: "127.0.0.1:4545".to_string(),
///     link: LinkCondition::default(),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct RelayArgs {
    /// The address and port to bind the TCP listener to (e.g., `127.0.0.1:4545`).
    pub bind_addr: String,
    /// The simulated network conditions for all connected clients.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_default_args() {
        let args = vec![];
        let parsed = parse_args(args).expect("Failed to parse empty args");
        assert_eq!(
            parsed.bind_addr, "127.0.0.1:4545",
            "Default bind address mismatch"
        );
        assert_eq!(parsed.link.latency_ms, 0, "Default latency mismatch");
        assert_eq!(parsed.link.jitter_ms, 0, "Default jitter mismatch");
        assert_eq!(parsed.link.loss_pct, 0, "Default loss mismatch");
        assert_eq!(parsed.link.reorder_pct, 0, "Default reorder mismatch");
    }

    #[test]
    fn should_return_help_error() {
        let args = vec!["--help".to_string()];
        let err = parse_args(args).expect_err("Expected error for --help");
        assert!(
            err.contains("Usage:"),
            "Help message should contain Usage instructions"
        );

        let args = vec!["-h".to_string()];
        let err = parse_args(args).expect_err("Expected error for -h");
        assert!(
            err.contains("Usage:"),
            "Help message should contain Usage instructions"
        );
    }

    #[test]
    fn should_parse_bind_arg() {
        let args = vec!["--bind".to_string(), "0.0.0.0:8080".to_string()];
        let parsed = parse_args(args).expect("Failed to parse --bind");
        assert_eq!(parsed.bind_addr, "0.0.0.0:8080", "Bind address mismatch");

        let args = vec!["--bind=192.168.1.1:9090".to_string()];
        let parsed = parse_args(args).expect("Failed to parse --bind=");
        assert_eq!(
            parsed.bind_addr, "192.168.1.1:9090",
            "Bind address mismatch"
        );
    }

    #[test]
    fn should_parse_latency_arg() {
        let args = vec!["--latency-ms".to_string(), "150".to_string()];
        let parsed = parse_args(args).expect("Failed to parse --latency-ms");
        assert_eq!(parsed.link.latency_ms, 150, "Latency mismatch");

        let args = vec!["--latency-ms=200".to_string()];
        let parsed = parse_args(args).expect("Failed to parse --latency-ms=");
        assert_eq!(parsed.link.latency_ms, 200, "Latency mismatch");
    }

    #[test]
    fn should_parse_jitter_arg() {
        let args = vec!["--jitter-ms".to_string(), "25".to_string()];
        let parsed = parse_args(args).expect("Failed to parse --jitter-ms");
        assert_eq!(parsed.link.jitter_ms, 25, "Jitter mismatch");

        let args = vec!["--jitter-ms=30".to_string()];
        let parsed = parse_args(args).expect("Failed to parse --jitter-ms=");
        assert_eq!(parsed.link.jitter_ms, 30, "Jitter mismatch");
    }

    #[test]
    fn should_parse_loss_pct_arg() {
        let args = vec!["--loss-pct".to_string(), "5".to_string()];
        let parsed = parse_args(args).expect("Failed to parse --loss-pct");
        assert_eq!(parsed.link.loss_pct, 5, "Loss percentage mismatch");

        let args = vec!["--loss-pct=10".to_string()];
        let parsed = parse_args(args).expect("Failed to parse --loss-pct=");
        assert_eq!(parsed.link.loss_pct, 10, "Loss percentage mismatch");
    }

    #[test]
    fn should_parse_reorder_pct_arg() {
        let args = vec!["--reorder-pct".to_string(), "15".to_string()];
        let parsed = parse_args(args).expect("Failed to parse --reorder-pct");
        assert_eq!(parsed.link.reorder_pct, 15, "Reorder percentage mismatch");

        let args = vec!["--reorder-pct=20".to_string()];
        let parsed = parse_args(args).expect("Failed to parse --reorder-pct=");
        assert_eq!(parsed.link.reorder_pct, 20, "Reorder percentage mismatch");
    }

    #[test]
    fn should_return_error_for_unknown_arg() {
        let args = vec!["--unknown".to_string()];
        let err = parse_args(args).expect_err("Expected error for unknown argument");
        assert!(
            err.contains("unknown argument '--unknown'"),
            "Error message mismatch"
        );
    }

    #[test]
    fn should_return_error_for_missing_value() {
        let args = vec!["--bind".to_string()];
        let err = parse_args(args).expect_err("Expected error for missing value");
        assert_eq!(err, "missing value after --bind", "Error message mismatch");

        let args = vec!["--latency-ms=".to_string()];
        let err = parse_args(args).expect_err("Expected error for missing value with =");
        assert_eq!(
            err, "missing value after --latency-ms=",
            "Error message mismatch"
        );
    }
}
