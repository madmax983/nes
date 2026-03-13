use std::{
    env,
    path::Path,
    sync::{Mutex, OnceLock},
};

use nes_ai::snapshot::{load_snapshot_bundle, write_snapshot_bundle};
use nes_core::NesCore;
use tempfile::tempdir;

#[test]
fn snapshot_bundle_round_trip_preserves_rom_hash_and_snapshot() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("control.state.json");

    let snapshot = NesCore::new().save_state();
    write_snapshot_bundle(&path, "rom-hash", "smb-control-v1", &snapshot).unwrap();

    let bundle = load_snapshot_bundle(&path).unwrap();
    assert_eq!(bundle.rom_hash, "rom-hash");
    assert_eq!(bundle.snapshot, snapshot);
    assert_eq!(bundle.snapshot_id, "smb-control-v1");
}

#[test]
fn snapshot_bundle_writes_to_cwd_relative_path() {
    let _lock = cwd_lock().lock().unwrap();
    let dir = tempdir().unwrap();
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();
    let _restore = RestoreCurrentDir(original_dir);

    let snapshot = NesCore::new().save_state();
    let path = Path::new("control.state.json");
    write_snapshot_bundle(path, "rom-hash", "smb-control-v1", &snapshot).unwrap();

    let bundle = load_snapshot_bundle(path).unwrap();
    assert_eq!(bundle.snapshot_id, "smb-control-v1");
}

fn cwd_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct RestoreCurrentDir(std::path::PathBuf);

impl Drop for RestoreCurrentDir {
    fn drop(&mut self) {
        env::set_current_dir(&self.0).unwrap();
    }
}
