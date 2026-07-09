use std::fs;
use std::path::Path;

use nes_config::{DEFAULT_CONFIG_PATH, NesConfig};

/// Resolves the absolute path to a Super Mario Bros. ROM file.
///
/// This reads the `roms.smb` path from `nes.toml` (or `desktop.rom_path` as a fallback)
/// to find a user-provided copy of SMB1. Panics if the config is missing or the file does not exist.
///
/// ## Panics
///
/// Panics if the `nes.toml` file is not configured with an SMB ROM path or if the file cannot be read.
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

/// Resolves the absolute path to the standard `nestest.nes` CPU validation ROM.
///
/// This reads the `roms.nestest` path from `nes.toml` to find the ROM file used
/// for validating CPU instruction execution and timing.
///
/// ## Panics
///
/// Panics if the `nes.toml` file is not configured with the nestest ROM path or if the file cannot be read.
#[allow(dead_code)]
pub fn nestest_rom_path() -> String {
    let config = load_config();
    let rom_path = config.roms.nestest.unwrap_or_else(|| {
        panic!("missing nestest ROM path in config; set `roms.nestest` in {DEFAULT_CONFIG_PATH}")
    });
    ensure_path_exists("NESTEST ROM", &rom_path)
}

/// Resolves the absolute path to Blargg's CPU test ROM.
///
/// This reads the `roms.blargg_cpu` path from `nes.toml`. This ROM is used for
/// deep verification of instruction timing and edge-case behavior.
///
/// ## Panics
///
/// Panics if the `nes.toml` file is not configured with the blargg CPU ROM path or if the file cannot be read.
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

/// Resolves a list of absolute paths to all ROMs in the bbbradsmith audio test suite.
///
/// This scans the directory specified by `roms.bbbradsmith_audio_suite_dir` in `nes.toml`
/// and returns paths to all `.nes` files found within, sorted alphabetically.
///
/// ## Panics
///
/// Panics if the `nes.toml` file is not configured with the suite directory path, if the
/// directory does not exist, or if the directory is completely empty.
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

/// Resolves the absolute path to the directory containing "golden" audio recordings.
///
/// This reads the `roms.bbbradsmith_audio_golden_dir` path from `nes.toml`. The golden
/// recordings are used as known-good standards to compare the emulator's audio output against.
///
/// ## Panics
///
/// Panics if the `nes.toml` file is not configured with the golden recordings directory path,
/// or if the directory does not exist.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires nes.toml or defaults to panic if absent in pure CI"]
    fn config_loads_without_panic() {
        let _ = load_config();
    }

    #[test]
    #[should_panic = "TEST path cannot be empty in nes.toml"]
    fn ensure_path_exists_panics_on_empty() {
        ensure_path_exists("TEST", " ");
    }

    #[test]
    #[should_panic = "TEST path does not exist: /does/not/exist/12345"]
    fn ensure_path_exists_panics_on_missing() {
        ensure_path_exists("TEST", "/does/not/exist/12345");
    }

    #[test]
    #[should_panic = "TEST path cannot be empty in nes.toml"]
    fn ensure_dir_exists_panics_on_empty() {
        ensure_dir_exists("TEST", "   ");
    }

    #[test]
    #[should_panic = "TEST does not exist: /does/not/exist/12345"]
    fn ensure_dir_exists_panics_on_missing() {
        ensure_dir_exists("TEST", "/does/not/exist/12345");
    }

    #[test]
    #[should_panic = "TEST is not a directory:"]
    fn ensure_dir_exists_panics_on_file() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let toml = Path::new(manifest).join("Cargo.toml");
        ensure_dir_exists("TEST", toml.to_string_lossy().as_ref());
    }

    #[test]
    #[ignore = "purely to verify panics locally for coverage without crashing pure CI"]
    fn cover_rom_path_helpers() {
        let _ = std::panic::catch_unwind(smb_rom_path);
        let _ = std::panic::catch_unwind(nestest_rom_path);
        let _ = std::panic::catch_unwind(blargg_cpu_rom_path);
        let _ = std::panic::catch_unwind(bbbradsmith_audio_suite_rom_paths);
        let _ = std::panic::catch_unwind(bbbradsmith_audio_golden_dir_path);
    }
}
