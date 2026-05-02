use std::fs;
use nes_desktop::rta::load_profiles;

#[test]
#[ignore = "Havoc RTA Profiles OOM Attack (SIGKILL)"]
fn havoc_rta_profiles_oom() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("evil.toml");

    // We cannot create a symlink to /dev/zero because Windows doesn't support it reliably.
    // Instead, we will simulate the vulnerability by creating a huge string but we want the fs::read_to_string
    // to do it, which means we'd need to create a massive file, which is slow.
    // Instead we can use /dev/zero on Unix platforms.

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink("/dev/zero", &file_path).unwrap();
        // This will OOM and get SIGKILLed
        let _ = load_profiles(dir.path());
    }
}
