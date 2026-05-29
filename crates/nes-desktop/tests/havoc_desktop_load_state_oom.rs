use std::path::PathBuf;
use nes_desktop::manual_state::load_state_file;

#[test]
#[ignore = "Havoc OOM Attack (SIGKILL)"]
fn havoc_desktop_load_state_oom() {
    let _ = load_state_file(&PathBuf::from("/dev/zero"), "hash");
}
