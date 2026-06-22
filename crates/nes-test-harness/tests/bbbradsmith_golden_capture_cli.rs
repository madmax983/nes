use std::process::Command;
use std::io::Write;

fn golden_capture_bin() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_bbbradsmith_golden_capture") {
        return std::path::PathBuf::from(path);
    }

    let mut dir = std::env::current_exe().expect("should locate test binary");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }

    let exe_name = if cfg!(windows) {
        "bbbradsmith_golden_capture.exe"
    } else {
        "bbbradsmith_golden_capture"
    };
    dir.join(exe_name)
}

#[test]
fn golden_capture_with_help_flag_prints_usage_and_succeeds() {
    let output = Command::new(golden_capture_bin())
        .arg("--help")
        .output()
        .expect("should run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: bbbradsmith_golden_capture"));
}

#[test]
fn golden_capture_with_invalid_arg_fails() {
    let output = Command::new(golden_capture_bin())
        .arg("--invalid")
        .output()
        .expect("should run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown argument"));
}

#[test]
fn golden_capture_with_bad_config_path_fails() {
    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg("does_not_exist_xyz.toml")
        .output()
        .expect("should run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to read config"));
}

#[test]
fn golden_capture_with_empty_config_fails() {
    let config_path = "empty_cfg_test_xyz.toml";
    let mut file = std::fs::File::create(config_path).unwrap();
    file.write_all(b"").unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(config_path)
        .output()
        .expect("should run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing `roms.bbbradsmith_audio_suite_dir`"));

    std::fs::remove_file(config_path).unwrap();
}

#[test]
fn golden_capture_with_invalid_suite_dir_fails() {
    let config_path = "invalid_dir_cfg_test_xyz.toml";
    let mut file = std::fs::File::create(config_path).unwrap();
    file.write_all(b"[roms]\nbbbradsmith_audio_suite_dir = \"does_not_exist\"\nbbbradsmith_audio_golden_dir = \"does_not_exist\"").unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(config_path)
        .output()
        .expect("should run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("directory does not exist or is not a directory"));

    std::fs::remove_file(config_path).unwrap();
}

#[test]
fn golden_capture_with_empty_suite_dir_fails() {
    let suite_dir = "suite_test_xyz";
    let golden_dir = "golden_test_xyz";
    std::fs::create_dir(suite_dir).unwrap_or_default();

    let config_path = "empty_suite_cfg_test_xyz.toml";
    let mut file = std::fs::File::create(config_path).unwrap();
    file.write_all(format!("[roms]\nbbbradsmith_audio_suite_dir = {:?}\nbbbradsmith_audio_golden_dir = {:?}", suite_dir, golden_dir).as_bytes()).unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(config_path)
        .output()
        .expect("should run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no .nes files found"));

    std::fs::remove_dir(suite_dir).unwrap();
    std::fs::remove_file(config_path).unwrap();
}

#[test]
fn golden_capture_with_valid_rom_writes_golden() {
    let suite_dir = "suite_test_xyz2";
    let golden_dir = "golden_test_xyz2";
    std::fs::create_dir(suite_dir).unwrap_or_default();
    std::fs::create_dir(golden_dir).unwrap_or_default();

    // We create a dummy .nes file
    let rom_path = format!("{suite_dir}/test.nes");
    // INES header + some bytes
    let mut rom_data = vec![0x4E, 0x45, 0x53, 0x1A, 0x01, 0x01, 0x00, 0x00, 0, 0, 0, 0, 0, 0, 0, 0];
    rom_data.resize(16 + 16384 + 8192, 0); // 1 PRG, 1 CHR
    std::fs::write(&rom_path, rom_data).unwrap();

    let config_path = "valid_cfg_test_xyz.toml";
    let mut file = std::fs::File::create(config_path).unwrap();
    file.write_all(format!("[roms]\nbbbradsmith_audio_suite_dir = {:?}\nbbbradsmith_audio_golden_dir = {:?}", suite_dir, golden_dir).as_bytes()).unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(config_path)
        .output()
        .expect("should run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Capturing bbbradsmith audio goldens"));
    assert!(stdout.contains("done: written="));

    // Now check if running again skips
    let output2 = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(config_path)
        .output()
        .expect("should run binary");

    assert!(output2.status.success());
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert!(stdout2.contains("skipped_existing="));
    assert!(stdout2.contains("1\u{1b}[39m")); // checking skipped_existing=1


    // Now check if force works
    let output3 = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(config_path)
        .arg("--force")
        .output()
        .expect("should run binary");

    assert!(output3.status.success());
    let stdout3 = String::from_utf8_lossy(&output3.stdout);
    assert!(stdout3.contains("done: written="));

    std::fs::remove_file(&rom_path).unwrap();
    std::fs::remove_dir(suite_dir).unwrap();
    std::fs::remove_file(format!("{golden_dir}/test.s16le.pcm")).unwrap();
    std::fs::remove_dir(golden_dir).unwrap();
    std::fs::remove_file(config_path).unwrap();
}

#[test]
fn golden_capture_with_force_flag_re_writes_golden() {
    let suite_dir = "suite_test_xyz3";
    let golden_dir = "golden_test_xyz3";
    std::fs::create_dir(suite_dir).unwrap_or_default();
    std::fs::create_dir(golden_dir).unwrap_or_default();

    // We create a dummy .nes file
    let rom_path = format!("{suite_dir}/test.nes");
    let mut rom_data = vec![0x4E, 0x45, 0x53, 0x1A, 0x01, 0x01, 0x00, 0x00, 0, 0, 0, 0, 0, 0, 0, 0];
    rom_data.resize(16 + 16384 + 8192, 0);
    std::fs::write(&rom_path, rom_data).unwrap();

    let config_path = "force_cfg_test_xyz.toml";
    let mut file = std::fs::File::create(config_path).unwrap();
    file.write_all(format!("[roms]\nbbbradsmith_audio_suite_dir = {:?}\nbbbradsmith_audio_golden_dir = {:?}", suite_dir, golden_dir).as_bytes()).unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(config_path)
        .output()
        .expect("should run binary");
    assert!(output.status.success());

    // Run again with another bad flag
    let output2 = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(config_path)
        .arg("--not-force")
        .output()
        .expect("should run binary");
    assert!(!output2.status.success());
    let stderr = String::from_utf8_lossy(&output2.stderr);
    assert!(stderr.contains("unknown argument '--not-force'"));

    // Run again with --force
    let output3 = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(config_path)
        .arg("--force")
        .output()
        .expect("should run binary");
    assert!(output3.status.success());
    let stdout = String::from_utf8_lossy(&output3.stdout);
    assert!(stdout.contains("done: written="));

    std::fs::remove_file(&rom_path).unwrap();
    std::fs::remove_dir(suite_dir).unwrap();
    std::fs::remove_file(format!("{golden_dir}/test.s16le.pcm")).unwrap();
    std::fs::remove_dir(golden_dir).unwrap();
    std::fs::remove_file(config_path).unwrap();
}
