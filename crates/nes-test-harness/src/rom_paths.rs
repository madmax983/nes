use std::fs;
use std::path::Path;

use nes_config::{DEFAULT_CONFIG_PATH, NesConfig};

#[allow(dead_code)]
pub fn smb_rom_path() -> String {
    let config = load_config();
    let rom_path = config
        .roms
        .smb
        .or(config.desktop.rom_path)
        .unwrap_or_else(|| {
            panic!(
                "missing SMB ROM path in config; set `roms.smb` or `desktop.rom_path` in {DEFAULT_CONFIG_PATH}"
            )
        });
    ensure_path_exists("SMB ROM", &rom_path)
}

#[allow(dead_code)]
pub fn nestest_rom_path() -> String {
    let config = load_config();
    let rom_path = config.roms.nestest.unwrap_or_else(|| {
        panic!("missing nestest ROM path in config; set `roms.nestest` in {DEFAULT_CONFIG_PATH}")
    });
    ensure_path_exists("NESTEST ROM", &rom_path)
}

#[allow(dead_code)]
pub fn blargg_cpu_rom_path() -> String {
    let config = load_config();
    let rom_path = config.roms.blargg_cpu.unwrap_or_else(|| {
        panic!(
            "missing blargg CPU ROM path in config; set `roms.blargg_cpu` in {DEFAULT_CONFIG_PATH}"
        )
    });
    ensure_path_exists("BLARGG CPU ROM", &rom_path)
}

#[allow(dead_code)]
pub fn bbbradsmith_audio_suite_rom_paths() -> Vec<String> {
    let config = load_config();
    let suite_dir = config.roms.bbbradsmith_audio_suite_dir.unwrap_or_else(|| {
        panic!(
            "missing bbbradsmith audio suite directory in config; set `roms.bbbradsmith_audio_suite_dir` in {DEFAULT_CONFIG_PATH}"
        )
    });
    ensure_dir_exists("bbbradsmith audio suite directory", &suite_dir);

    let mut rom_paths = fs::read_dir(&suite_dir)
        .unwrap_or_else(|err| panic!("unable to read suite directory '{suite_dir}': {err}"))
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("nes"))
        })
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    rom_paths.sort_unstable_by_key(|path| path.to_ascii_lowercase());

    assert!(
        !rom_paths.is_empty(),
        "bbbradsmith audio suite directory is empty (no .nes files found): {suite_dir}"
    );
    rom_paths
}

#[allow(dead_code)]
pub fn bbbradsmith_audio_golden_dir_path() -> String {
    let config = load_config();
    let golden_dir = config.roms.bbbradsmith_audio_golden_dir.unwrap_or_else(|| {
        panic!(
            "missing bbbradsmith golden directory in config; set `roms.bbbradsmith_audio_golden_dir` in {DEFAULT_CONFIG_PATH}"
        )
    });
    ensure_dir_exists("bbbradsmith golden directory", &golden_dir);
    golden_dir
}

fn load_config() -> NesConfig {
    let workspace_config = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(DEFAULT_CONFIG_PATH);
    if workspace_config.exists() {
        return NesConfig::load(&workspace_config).unwrap_or_else(|err| {
            panic!(
                "unable to load config '{}': {err}",
                workspace_config.display()
            )
        });
    }
    NesConfig::load_or_default(None).unwrap_or_else(|err| panic!("unable to load config: {err}"))
}

fn ensure_path_exists(label: &str, path: &str) -> String {
    assert!(
        !path.trim().is_empty(),
        "{label} path cannot be empty in {DEFAULT_CONFIG_PATH}"
    );
    assert!(
        Path::new(path).exists(),
        "{label} path does not exist: {path}"
    );
    path.to_owned()
}

fn ensure_dir_exists(label: &str, path: &str) {
    assert!(
        !path.trim().is_empty(),
        "{label} path cannot be empty in {DEFAULT_CONFIG_PATH}"
    );
    let dir = Path::new(path);
    assert!(dir.exists(), "{label} does not exist: {path}");
    assert!(dir.is_dir(), "{label} is not a directory: {path}");
}
