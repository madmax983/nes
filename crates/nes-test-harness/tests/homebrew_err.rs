use std::fs;
use nes_test_harness::write_homebrew_rom;

#[test]
fn write_homebrew_rom_fails_on_bad_path() {
    let temp_dir = std::env::temp_dir().join("nes_test_harness_homebrew_err_test");
    // writing to a path that is just a directory instead of a file
    let _ = fs::create_dir_all(&temp_dir);

    let result = write_homebrew_rom(&temp_dir);
    assert!(result.is_err());

    let bad_path = temp_dir.join("foo/bar/baz.nes");
    // write a file where the directory should be
    let _ = fs::write(&temp_dir.join("foo"), "not a dir");
    let result = write_homebrew_rom(&bad_path);
    assert!(result.is_err());

    let _ = fs::remove_dir_all(&temp_dir);
}
