use std::path::Path;

#[test]
#[ignore = "Havoc OOM Attack (SIGKILL)"]
fn havoc_desktop_load_rom_oom() {
    let mut core = nes_core::NesCore::new();
    // the module `session` is internal to the nes-desktop bin crate `main.rs`,
    // so we can't test it directly from `tests/`, but we can test manual_state since that's public!
    let _ = nes_desktop::manual_state::load_state_file(Path::new("/dev/zero"), "hash");
}
