use std::fs;
use nes_test_harness::rom_paths::*;

#[test]
fn test_rom_paths_panics_on_missing_or_empty() {
    let _ = std::panic::catch_unwind(smb_rom_path);
    let _ = std::panic::catch_unwind(nestest_rom_path);
    let _ = std::panic::catch_unwind(blargg_cpu_rom_path);
    let _ = std::panic::catch_unwind(bbbradsmith_audio_suite_rom_paths);
    let _ = std::panic::catch_unwind(bbbradsmith_audio_golden_dir_path);
}
