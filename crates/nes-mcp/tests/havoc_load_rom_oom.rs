use nes_core::NesCore;
use nes_mcp::ToolParams;
use nes_mcp::dispatch_tool;

#[test]
fn havoc_load_rom_oom() {
    let mut core = NesCore::new();
    let mut params = ToolParams::new();
    params.insert("rom_path".to_string(), "/dev/zero".to_string());

    // Reading from /dev/zero using std::fs::read will allocate memory
    // until the system runs out of RAM and sends a SIGKILL to the process.
    // By using bounded_read, we expect a DispatchError instead.
    let result = dispatch_tool(&mut core, "load_rom", &params);
    assert!(result.is_err(), "Expected an error from reading /dev/zero");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("exceeds maximum size"),
        "Expected size limit error, got: {}",
        err_msg
    );
}
