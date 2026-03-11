use crate::dispatch::{DispatchError, DispatchOutput, ToolParams};
use nes_core::NesCore;
use nes_core::experimental::scanner::{MemoryScanner, ScanCondition};

pub fn handle_scan_memory(
    core: &NesCore,
    scanner: &mut MemoryScanner,
    params: &ToolParams,
) -> Result<DispatchOutput, DispatchError> {
    let condition_str = params
        .get("condition")
        .map(String::as_str)
        .ok_or_else(|| DispatchError::InvalidParams("condition must be provided".to_owned()))?;

    let condition = match condition_str.to_ascii_lowercase().as_str() {
        "exact" => {
            let value_str = params.get("value").ok_or_else(|| {
                DispatchError::InvalidParams(
                    "value must be provided when condition is 'exact'".to_owned(),
                )
            })?;
            let value = parse_u8(value_str)?;
            ScanCondition::Exact(value)
        }
        "decreased" => ScanCondition::Decreased,
        "increased" => ScanCondition::Increased,
        "changed" => ScanCondition::Changed,
        "unchanged" => ScanCondition::Unchanged,
        _ => {
            return Err(DispatchError::InvalidParams(format!(
                "unknown scan condition '{}'",
                condition_str
            )));
        }
    };

    scanner.scan(core, condition);

    let candidate_count = scanner.candidate_count();
    let mut candidates = Vec::new();

    // Only return the actual list if it's small enough to not blow up JSON RPC
    if candidate_count <= 100 {
        candidates = scanner.candidates();
    }

    Ok(DispatchOutput::ScanResults {
        candidate_count,
        candidates,
    })
}

pub fn handle_reset_scanner(scanner: &mut MemoryScanner) -> Result<DispatchOutput, DispatchError> {
    scanner.reset();

    Ok(DispatchOutput::ScanResults {
        candidate_count: 0,
        candidates: Vec::new(),
    })
}

fn parse_u8(raw: &str) -> Result<u8, DispatchError> {
    if let Some(stripped) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        u8::from_str_radix(stripped, 16)
            .map_err(|_| DispatchError::InvalidParams(format!("invalid hex value '{}'", raw)))
    } else {
        raw.parse::<u8>()
            .map_err(|_| DispatchError::InvalidParams(format!("invalid integer value '{}'", raw)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn map_params(kvs: &[(&str, &str)]) -> ToolParams {
        let mut map = BTreeMap::new();
        for &(k, v) in kvs {
            map.insert(k.to_owned(), v.to_owned());
        }
        map
    }

    #[test]
    fn test_parse_u8() {
        assert_eq!(parse_u8("123").unwrap(), 123);
        assert_eq!(parse_u8("0xff").unwrap(), 255);
        assert_eq!(parse_u8("0X10").unwrap(), 16);
        assert!(parse_u8("256").is_err());
        assert!(parse_u8("0xGG").is_err());
    }

    #[test]
    fn test_scan_memory_exact() {
        let mut core = NesCore::new();
        let mut scanner = MemoryScanner::new();
        // Setup initial scanner memory
        core.load_cpu_bytes(0x0042, &[42]);
        core.load_cpu_bytes(0x0100, &[42]);

        handle_reset_scanner(&mut scanner).unwrap();

        // Perform first scan
        let res = handle_scan_memory(
            &core,
            &mut scanner,
            &map_params(&[("condition", "exact"), ("value", "42")]),
        )
        .unwrap();

        let (count, cands) = match res {
            DispatchOutput::ScanResults {
                candidate_count,
                candidates,
            } => (candidate_count, candidates),
            _ => panic!("Expected ScanResults"),
        };

        assert!(count >= 2);
        assert!(cands.contains(&0x0042));
        assert!(cands.contains(&0x0100));

        // Mutate one value
        core.load_cpu_bytes(0x0100, &[43]);

        // Second scan
        let res2 = handle_scan_memory(
            &core,
            &mut scanner,
            &map_params(&[("condition", "exact"), ("value", "42")]),
        )
        .unwrap();
        let (count2, cands2) = match res2 {
            DispatchOutput::ScanResults {
                candidate_count,
                candidates,
            } => (candidate_count, candidates),
            _ => panic!("Expected ScanResults"),
        };

        assert!(count2 < count);
        assert!(cands2.contains(&0x0042));
        assert!(!cands2.contains(&0x0100));
    }

    #[test]
    fn test_scan_memory_relative() {
        let mut core = NesCore::new();
        let mut scanner = MemoryScanner::new();
        core.load_cpu_bytes(0x0050, &[10]);

        handle_reset_scanner(&mut scanner).unwrap();

        // Initial scan (all variables added)
        handle_scan_memory(
            &core,
            &mut scanner,
            &map_params(&[("condition", "exact"), ("value", "10")]),
        )
        .unwrap();

        // Mutate
        core.load_cpu_bytes(0x0050, &[9]); // decreased

        let res = handle_scan_memory(
            &core,
            &mut scanner,
            &map_params(&[("condition", "decreased")]),
        )
        .unwrap();
        let cands = match res {
            DispatchOutput::ScanResults { candidates, .. } => candidates,
            _ => panic!("Expected ScanResults"),
        };
        assert!(cands.contains(&0x0050));
    }
}
