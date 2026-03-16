use std::env;
use std::fs;
use std::path::PathBuf;

use nes_core::NesCore;
use nes_mcp::{dispatch_tool, ToolParams};

fn params(pairs: &[(&str, &str)]) -> ToolParams {
    let mut map = ToolParams::new();
    for (key, value) in pairs {
        map.insert((*key).to_owned(), (*value).to_owned());
    }
    map
}

struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn havoc_test_absolute_path() {
    let mut core = NesCore::new();
    let abs_path = env::temp_dir().join("havoc_abs_test.nes");
    let path_str = abs_path.to_string_lossy().to_string();

    // Setup cleanup in case the test panics (or succeeds)
    let _cleanup = Cleanup(abs_path.clone());

    let output = dispatch_tool(
        &mut core,
        "export_6502_dsl_rom",
        &params(&[
            (
                "source",
                ".bank 1\nRESET:\n  LDA #$11\n  JMP RESET\n.reset RESET\n",
            ),
            ("output_path", &path_str),
        ]),
    );

    // If it is Ok, Havoc successfully proved it could write an absolute path.
    // We expect it to be blocked (Err), so if it works, Havoc wins (and the test panics).
    assert!(output.is_err(), "Absolute paths should have been blocked");
}

#[test]
fn havoc_test_parent_traversal() {
    let mut core = NesCore::new();
    // Build a path that explicitly uses `..` but resolves into the safe temp dir
    // e.g. /tmp/havoc_dir/../havoc_parent_test.nes
    let base_dir = env::temp_dir().join("havoc_dir");
    let _ = fs::create_dir_all(&base_dir);

    let parent_path = base_dir.join("..").join("havoc_parent_test.nes");
    let path_str = parent_path.to_string_lossy().to_string();

    // Setup cleanup
    let _cleanup = Cleanup(parent_path.clone());
    let _cleanup_dir = Cleanup(base_dir.clone());

    let output = dispatch_tool(
        &mut core,
        "export_6502_dsl_rom",
        &params(&[
            (
                "source",
                ".bank 1\nRESET:\n  LDA #$11\n  JMP RESET\n.reset RESET\n",
            ),
            ("output_path", &path_str),
        ]),
    );

    assert!(output.is_err(), "Parent traversal paths should have been blocked");
}
