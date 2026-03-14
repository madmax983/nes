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
        if arg == "--bind" {
            let Some(value) = args.get(idx + 1) else {
                return Err("missing value after --bind".to_owned());
            };
            parsed.bind_addr = value.clone();
            idx += 2;
            continue;
        }
        if arg == "--latency-ms" {
            let Some(value) = args.get(idx + 1) else {
                return Err("missing value after --latency-ms".to_owned());
            };
            parsed.link.latency_ms = parse_u64_arg(value, "--latency-ms")?;
            idx += 2;
            continue;
        }
        if arg == "--jitter-ms" {
            let Some(value) = args.get(idx + 1) else {
                return Err("missing value after --jitter-ms".to_owned());
            };
            parsed.link.jitter_ms = parse_u64_arg(value, "--jitter-ms")?;
            idx += 2;
            continue;
        }
        if arg == "--loss-pct" {
            let Some(value) = args.get(idx + 1) else {
                return Err("missing value after --loss-pct".to_owned());
            };
            parsed.link.loss_pct = parse_percent_arg(value, "--loss-pct")?;
            idx += 2;
            continue;
        }
        if arg == "--reorder-pct" {
            let Some(value) = args.get(idx + 1) else {
                return Err("missing value after --reorder-pct".to_owned());
            };
            parsed.link.reorder_pct = parse_percent_arg(value, "--reorder-pct")?;
            idx += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--bind=") {
            if value.is_empty() {
                return Err("missing value after --bind=".to_owned());
            }
            parsed.bind_addr = value.to_owned();
            idx += 1;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--latency-ms=") {
            if value.is_empty() {
                return Err("missing value after --latency-ms=".to_owned());
            }
            parsed.link.latency_ms = parse_u64_arg(value, "--latency-ms")?;
            idx += 1;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--jitter-ms=") {
            if value.is_empty() {
                return Err("missing value after --jitter-ms=".to_owned());
            }
            parsed.link.jitter_ms = parse_u64_arg(value, "--jitter-ms")?;
            idx += 1;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--loss-pct=") {
            if value.is_empty() {
                return Err("missing value after --loss-pct=".to_owned());
            }
            parsed.link.loss_pct = parse_percent_arg(value, "--loss-pct")?;
            idx += 1;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--reorder-pct=") {
            if value.is_empty() {
                return Err("missing value after --reorder-pct=".to_owned());
            }
            parsed.link.reorder_pct = parse_percent_arg(value, "--reorder-pct")?;
            idx += 1;
            continue;
        }
        return Err(format!(
            "unknown argument '{arg}'. Usage: nes-relay [--bind <addr>] [--latency-ms <n>] [--jitter-ms <n>] [--loss-pct <0..100>] [--reorder-pct <0..100>]"
        ));
    }
    Ok(parsed)
}

pub fn parse_u64_arg(value: &str, flag: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{flag} must be a non-negative integer"))
}

pub fn parse_percent_arg(value: &str, flag: &str) -> Result<u8, String> {
    let parsed = value
        .parse::<u8>()
        .map_err(|_| format!("{flag} must be an integer in [0, 100]"))?;
    if parsed > 100 {
        return Err(format!("{flag} must be in [0, 100]"));
    }
    Ok(parsed)
}
