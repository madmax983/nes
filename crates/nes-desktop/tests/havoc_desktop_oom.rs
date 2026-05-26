#![cfg(feature = "mcp-host")]
use nes_desktop::manual_state::load_state_file;

#[test]
#[ignore = "Havoc OOM Attack (SIGKILL)"]
fn havoc_desktop_load_state_oom() {
    // The Trigger: passing /dev/zero to load_state_file will cause an OOM SIGKILL.
    // It blindly uses fs::read.
    let _ = load_state_file(std::path::Path::new("/dev/zero"), "hash");
}
