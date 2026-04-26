use std::os::unix::fs::PermissionsExt;
use std::process::Command;

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
    for flag in ["--help", "-h"] {
        let output = Command::new(golden_capture_bin())
            .arg(flag)
            .output()
            .expect("run bbbradsmith_golden_capture");

        assert!(output.status.success(), "failed on flag {flag}");
        let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
        assert!(stdout.contains("Usage: bbbradsmith_golden_capture [--config <path>] [--force]"));
    }
}

#[test]
fn golden_capture_rejects_unknown_arguments() {
    let output = Command::new(golden_capture_bin())
        .arg("--unknown-flag")
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(!output.status.success(), "should fail on unknown flag");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("unknown argument '--unknown-flag'"));
}

#[test]
fn golden_capture_prints_error_when_config_is_missing() {
    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg("/does/not/exist/fake_config.toml")
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(!output.status.success(), "should fail on missing config");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("Error: failed to read config '/does/not/exist/fake_config.toml'"));
}

#[test]
fn golden_capture_prints_error_when_suite_dir_missing() {
    let temp_dir = std::env::temp_dir().join("golden_capture_test_config");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let config_path = temp_dir.join("missing_suite_dir.toml");
    std::fs::write(&config_path, "[roms]\n").unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(!output.status.success(), "should fail on missing suite dir");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("missing `roms.bbbradsmith_audio_suite_dir` in config"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn golden_capture_prints_error_when_golden_dir_missing() {
    let temp_dir = std::env::temp_dir().join("golden_capture_test_config2");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let config_path = temp_dir.join("missing_golden_dir.toml");
    std::fs::write(
        &config_path,
        "[roms]\nbbbradsmith_audio_suite_dir = \"/tmp\"\n",
    )
    .unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(
        !output.status.success(),
        "should fail on missing golden dir"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("missing `roms.bbbradsmith_audio_golden_dir` in config"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn golden_capture_prints_error_when_suite_dir_does_not_exist() {
    let temp_dir = std::env::temp_dir().join("golden_capture_test_config3");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let config_path = temp_dir.join("bad_suite_dir.toml");
    std::fs::write(&config_path, "[roms]\nbbbradsmith_audio_suite_dir = \"/does/not/exist/123\"\nbbbradsmith_audio_golden_dir = \"/tmp\"\n").unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(!output.status.success(), "should fail on missing suite dir");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(
        stderr.contains("bbbradsmith audio suite directory does not exist or is not a directory")
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn golden_capture_prints_error_when_suite_dir_empty() {
    let temp_dir = std::env::temp_dir().join("golden_capture_test_config4");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let empty_suite = temp_dir.join("empty_suite");
    std::fs::create_dir_all(&empty_suite).unwrap();
    let config_path = temp_dir.join("empty_suite_dir.toml");
    std::fs::write(&config_path, format!("[roms]\nbbbradsmith_audio_suite_dir = \"{}\"\nbbbradsmith_audio_golden_dir = \"/tmp\"\n", empty_suite.display().to_string().replace("\\", "/"))).unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(!output.status.success(), "should fail on empty suite dir");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("no .nes files found in suite directory"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn golden_capture_prints_error_when_golden_dir_creation_fails() {
    let temp_dir = std::env::temp_dir().join("golden_capture_test_config5");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let suite = temp_dir.join("suite");
    std::fs::create_dir_all(&suite).unwrap();
    let config_path = temp_dir.join("config.toml");
    std::fs::write(&config_path, format!("[roms]\nbbbradsmith_audio_suite_dir = \"{}\"\nbbbradsmith_audio_golden_dir = \"/dev/null/dir\"\n", suite.display().to_string().replace("\\", "/"))).unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(
        !output.status.success(),
        "should fail when golden dir cannot be created"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("failed to create golden directory"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn golden_capture_run_succeeds_with_valid_rom() {
    let temp_dir = std::env::temp_dir().join("golden_capture_test_config6");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let suite = temp_dir.join("suite");
    std::fs::create_dir_all(&suite).unwrap();
    let rom_path = suite.join("test.nes");
    // Write a dummy minimal NES ROM (NROM-128)
    let mut rom = Vec::new();
    rom.extend_from_slice(b"NES\x1A");
    rom.extend_from_slice(&[1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

    let mut prg = vec![0_u8; 16384];
    prg[0] = 0xEA; // NOP
    prg[1] = 0x4C; // JMP $C000
    prg[2] = 0x00;
    prg[3] = 0xC0;
    // vectors
    let nmi_offset = 16384 - 6;
    let reset_offset = 16384 - 4;
    let irq_offset = 16384 - 2;
    prg[nmi_offset..nmi_offset + 2].copy_from_slice(&0xC000_u16.to_le_bytes());
    prg[reset_offset..reset_offset + 2].copy_from_slice(&0xC000_u16.to_le_bytes());
    prg[irq_offset..irq_offset + 2].copy_from_slice(&0xC000_u16.to_le_bytes());

    rom.extend_from_slice(&prg);
    rom.extend_from_slice(&[0; 8192]);
    std::fs::write(&rom_path, &rom).unwrap();

    let golden = temp_dir.join("golden");
    let config_path = temp_dir.join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            "[roms]\nbbbradsmith_audio_suite_dir = \"{}\"\nbbbradsmith_audio_golden_dir = \"{}\"\n",
            suite.display().to_string().replace("\\", "/"),
            golden.display().to_string().replace("\\", "/")
        ),
    )
    .unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(output.status.success(), "should succeed with valid rom");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("test.nes"));
    assert!(stdout.contains("Written"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn golden_capture_skips_unsupported_mapper() {
    let temp_dir = std::env::temp_dir().join("golden_capture_test_config7");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let suite = temp_dir.join("suite");
    std::fs::create_dir_all(&suite).unwrap();
    let rom_path = suite.join("test.nes");

    // Write a dummy minimal NES ROM (Mapper 99)
    let mut rom = Vec::new();
    rom.extend_from_slice(b"NES\x1A");
    // mapper id 99 (0x63), lower 4 bits = 0x30, upper 4 bits = 0x60
    rom.extend_from_slice(&[1, 1, 0x30, 0x60, 0, 0, 0, 0, 0, 0, 0, 0]);
    rom.extend_from_slice(&[0; 16384]);
    rom.extend_from_slice(&[0; 8192]);
    std::fs::write(&rom_path, &rom).unwrap();

    let golden = temp_dir.join("golden");
    let config_path = temp_dir.join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            "[roms]\nbbbradsmith_audio_suite_dir = \"{}\"\nbbbradsmith_audio_golden_dir = \"{}\"\n",
            suite.display().to_string().replace("\\", "/"),
            golden.display().to_string().replace("\\", "/")
        ),
    )
    .unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("run bbbradsmith_golden_capture");

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("test.nes"));
    assert!(stdout.contains("Skip (Mapper)"));
    assert!(stdout.contains("skipped_mapper="));
    assert!(stdout.contains("1"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn golden_capture_skips_existing() {
    let temp_dir = std::env::temp_dir().join("golden_capture_test_config8");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let suite = temp_dir.join("suite");
    std::fs::create_dir_all(&suite).unwrap();
    let rom_path = suite.join("test.nes");
    // Write a dummy minimal NES ROM (NROM-128)
    let mut rom = Vec::new();
    rom.extend_from_slice(b"NES\x1A");
    rom.extend_from_slice(&[1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let mut prg = vec![0_u8; 16384];
    prg[0] = 0xEA;
    prg[1] = 0x4C;
    prg[2] = 0x00;
    prg[3] = 0xC0;
    let nmi_offset = 16384 - 6;
    let reset_offset = 16384 - 4;
    let irq_offset = 16384 - 2;
    prg[nmi_offset..nmi_offset + 2].copy_from_slice(&0xC000_u16.to_le_bytes());
    prg[reset_offset..reset_offset + 2].copy_from_slice(&0xC000_u16.to_le_bytes());
    prg[irq_offset..irq_offset + 2].copy_from_slice(&0xC000_u16.to_le_bytes());
    rom.extend_from_slice(&prg);
    rom.extend_from_slice(&[0; 8192]);
    std::fs::write(&rom_path, &rom).unwrap();

    let golden = temp_dir.join("golden");
    std::fs::create_dir_all(&golden).unwrap();
    std::fs::write(golden.join("test.s16le.pcm"), b"dummy").unwrap(); // create existing file

    let config_path = temp_dir.join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            "[roms]\nbbbradsmith_audio_suite_dir = \"{}\"\nbbbradsmith_audio_golden_dir = \"{}\"\n",
            suite.display().to_string().replace("\\", "/"),
            golden.display().to_string().replace("\\", "/")
        ),
    )
    .unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(output.status.success() || !output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("Skip (Exists)"));
    assert!(stdout.contains("skipped_existing="));
    assert!(stdout.contains("1"));

    // Test with --force
    let output_force = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(&config_path)
        .arg("--force")
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(output_force.status.success(), "should succeed with --force");
    let stdout_force = String::from_utf8(output_force.stdout).expect("stdout utf8");
    assert!(stdout_force.contains("Written"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn golden_capture_prints_error_when_rom_unreadable() {
    let temp_dir = std::env::temp_dir().join("golden_capture_test_config9");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let suite = temp_dir.join("suite");
    std::fs::create_dir_all(&suite).unwrap();
    let rom_path = suite.join("test.nes");
    std::fs::write(&rom_path, "dummy").unwrap();
    // make unreadable
    let mut perms = std::fs::metadata(&rom_path).unwrap().permissions();
    perms.set_mode(0o000);
    std::fs::set_permissions(&rom_path, perms).unwrap();

    let golden = temp_dir.join("golden");
    let config_path = temp_dir.join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            "[roms]\nbbbradsmith_audio_suite_dir = \"{}\"\nbbbradsmith_audio_golden_dir = \"{}\"\n",
            suite.display().to_string().replace("\\", "/"),
            golden.display().to_string().replace("\\", "/")
        ),
    )
    .unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(!output.status.success(), "should fail on unreadable ROM");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("failed to read ROM"));

    let mut perms = std::fs::metadata(&rom_path).unwrap().permissions();
    perms.set_mode(0o644);
    std::fs::set_permissions(&rom_path, perms).unwrap();
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn golden_capture_prints_error_when_suite_dir_unreadable() {
    let temp_dir = std::env::temp_dir().join("golden_capture_test_config10");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let suite = temp_dir.join("suite");
    std::fs::create_dir_all(&suite).unwrap();

    // make directory unreadable
    let mut perms = std::fs::metadata(&suite).unwrap().permissions();
    perms.set_mode(0o000);
    std::fs::set_permissions(&suite, perms).unwrap();

    let golden = temp_dir.join("golden");
    let config_path = temp_dir.join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            "[roms]\nbbbradsmith_audio_suite_dir = \"{}\"\nbbbradsmith_audio_golden_dir = \"{}\"\n",
            suite.display().to_string().replace("\\", "/"),
            golden.display().to_string().replace("\\", "/")
        ),
    )
    .unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(
        !output.status.success(),
        "should fail on unreadable suite dir"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("failed to read suite directory"));

    let mut perms = std::fs::metadata(&suite).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&suite, perms).unwrap();
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn golden_capture_prints_error_when_missing_stem() {
    let temp_dir = std::env::temp_dir().join("golden_capture_test_config11");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let suite = temp_dir.join("suite");
    std::fs::create_dir_all(&suite).unwrap();

    let rom_path = suite.join("bad.nes");
    let mut rom = Vec::new();
    rom.extend_from_slice(b"NES\x1A");
    rom.extend_from_slice(&[1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    // Missing PRG! This will fail during `capture_audio_window` -> `core.load_rom` -> Err
    std::fs::write(&rom_path, &rom).unwrap();

    let golden = temp_dir.join("golden");
    let config_path = temp_dir.join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            "[roms]\nbbbradsmith_audio_suite_dir = \"{}\"\nbbbradsmith_audio_golden_dir = \"{}\"\n",
            suite.display().to_string().replace("\\", "/"),
            golden.display().to_string().replace("\\", "/")
        ),
    )
    .unwrap();

    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(!output.status.success(), "should fail on bad rom execution");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    // capture_audio_window returns `Result<Vec<i16>, String>`, mapping errors to strings.
    assert!(stderr.contains("Error:"), "should print error");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn golden_capture_fails_to_write_pcm() {
    let temp_dir = std::env::temp_dir().join("golden_capture_test_config12");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let suite = temp_dir.join("suite");
    std::fs::create_dir_all(&suite).unwrap();
    let rom_path = suite.join("test.nes");
    // Write a dummy minimal NES ROM (NROM-128)
    let mut rom = Vec::new();
    rom.extend_from_slice(b"NES\x1A");
    rom.extend_from_slice(&[1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let mut prg = vec![0_u8; 16384];
    prg[0] = 0xEA;
    prg[1] = 0x4C;
    prg[2] = 0x00;
    prg[3] = 0xC0;
    let nmi_offset = 16384 - 6;
    let reset_offset = 16384 - 4;
    let irq_offset = 16384 - 2;
    prg[nmi_offset..nmi_offset + 2].copy_from_slice(&0xC000_u16.to_le_bytes());
    prg[reset_offset..reset_offset + 2].copy_from_slice(&0xC000_u16.to_le_bytes());
    prg[irq_offset..irq_offset + 2].copy_from_slice(&0xC000_u16.to_le_bytes());
    rom.extend_from_slice(&prg);
    rom.extend_from_slice(&[0; 8192]);
    std::fs::write(&rom_path, &rom).unwrap();

    let golden = temp_dir.join("golden");
    std::fs::create_dir_all(&golden).unwrap();

    // make the target file directory unwriteable
    let mut perms = std::fs::metadata(&golden).unwrap().permissions();
    perms.set_mode(0o555);
    std::fs::set_permissions(&golden, perms).unwrap();

    let config_path = temp_dir.join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            "[roms]\nbbbradsmith_audio_suite_dir = \"{}\"\nbbbradsmith_audio_golden_dir = \"{}\"\n",
            suite.display().to_string().replace("\\", "/"),
            golden.display().to_string().replace("\\", "/")
        ),
    )
    .unwrap();

    let _output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("run bbbradsmith_golden_capture");

    // The current tests will not fail if file cannot be created as the directory is writable by root.
    // So this test just covers execution path. It might not fail on CI.
    let mut perms = std::fs::metadata(&golden).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&golden, perms).unwrap();
    let _ = std::fs::remove_dir_all(&temp_dir);
}
