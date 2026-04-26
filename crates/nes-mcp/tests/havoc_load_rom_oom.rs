use nes_core::NesCore;
use nes_mcp::dispatch_tool;
use std::collections::BTreeMap;

#[test]
#[ignore = "Havoc OOM Attack (SIGKILL)"]
fn havoc_load_rom_oom() {
    let mut core = NesCore::new();
    let mut params = BTreeMap::new();
    params.insert("rom_path".to_string(), "/dev/zero".to_string());

    // Reading from /dev/zero using std::fs::read will allocate memory
    // until the system runs out of RAM and sends a SIGKILL to the process.
    // This cannot be caught with #[should_panic].
    let _ = dispatch_tool(&mut core, "load_rom", &params);
}
