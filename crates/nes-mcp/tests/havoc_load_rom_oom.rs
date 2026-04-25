use nes_core::NesCore;
use nes_mcp::{ToolParams, dispatch_tool};

#[test]
#[ignore = "Havoc OOM Target"]
fn havoc_test_load_rom_oom() {
    let mut core = NesCore::new();
    let mut params = ToolParams::new();
    params.insert("rom_path".to_string(), "/dev/zero".to_string());

    // This will read from /dev/zero indefinitely and eventually OOM the system.
    let _ = dispatch_tool(&mut core, "load_rom", &params);
}
