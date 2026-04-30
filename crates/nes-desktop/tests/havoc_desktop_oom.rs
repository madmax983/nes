use nes_desktop::manual_state::load_state_file;

#[test]
fn havoc_desktop_load_state_oom() {
    // The Trigger: passing /dev/zero to load_state_file will cause an OOM SIGKILL.
    // By using a bounded read, this should return a capacity error instead of crashing.
    let result = load_state_file(std::path::Path::new("/dev/zero"), "hash");
    assert!(result.is_err(), "Expected an error from reading /dev/zero");
    assert!(
        result.unwrap_err().contains("exceeds maximum size"),
        "Expected size limit error"
    );
}
