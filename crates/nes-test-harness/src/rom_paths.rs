//! Common paths for test ROMs and resources used by the integration test suite.
//!
//! This module abstracts the lookup of configured ROM paths, allowing tests to access them without
//! repeatedly parsing the workspace `nes.toml` configuration. It also provides validation to ensure
//! tests fail quickly if a required resource is missing.

use std::fs;
use std::path::Path;

use nes_config::{DEFAULT_CONFIG_PATH, NesConfig};

/// Returns the path to the Super Mario Bros. ROM.
///
/// This checks `roms.smb` first, falling back to `desktop.rom_path` in `nes.toml`.
///
/// ## Panics
/// Panics if the configuration is missing or the file does not exist.
///
/// ## Examples
///
/// ```rust
/// use std::panic::catch_unwind;
/// use nes_test_harness::rom_paths::smb_rom_path;
///
/// // Example of attempting to read the ROM path in a test
/// let result = catch_unwind(|| smb_rom_path());
/// ```
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

/// Returns the path to the `nestest` ROM.
///
/// Looks up `roms.nestest` in the workspace `nes.toml`.
///
/// ## Panics
/// Panics if the configuration is missing or the file does not exist.
#[allow(dead_code)]
pub fn nestest_rom_path() -> String {
    let config = load_config();
    let rom_path = config.roms.nestest.unwrap_or_else(|| {
        panic!("missing nestest ROM path in config; set `roms.nestest` in {DEFAULT_CONFIG_PATH}")
    });
    ensure_path_exists("NESTEST ROM", &rom_path)
}

/// Returns the path to the blargg CPU timing ROM.
///
/// Looks up `roms.blargg_cpu` in the workspace `nes.toml`.
///
/// ## Panics
/// Panics if the configuration is missing or the file does not exist.
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

/// Returns a sorted list of paths to all NES ROMs in the bbbradsmith audio suite.
///
/// This checks `roms.bbbradsmith_audio_suite_dir` in `nes.toml` and returns all `.nes` files within it.
///
/// ## Panics
/// Panics if the directory is missing, cannot be read, or contains no `.nes` files.
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

/// Returns the path to the directory containing golden reference audio for the bbbradsmith suite.
///
/// Looks up `roms.bbbradsmith_audio_golden_dir` in the workspace `nes.toml`.
///
/// ## Panics
/// Panics if the configuration is missing or the directory does not exist.
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
