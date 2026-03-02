#[cfg(loom)]
use loom::sync::{Arc, Mutex};
#[cfg(not(loom))]
use std::sync::{Arc, Mutex};

#[cfg(loom)]
use loom::thread;
#[cfg(not(loom))]
use std::thread;

use nes_netplay::{RollbackConfig, RollbackEngine};

#[test]
fn test_rollback_concurrency() {
    #[cfg(loom)]
    loom::model(|| {
        let config = RollbackConfig {
            local_player: 1,
            input_delay_frames: 0,
            max_rollback_frames: 60,
        };
        let engine = Arc::new(Mutex::new(RollbackEngine::new(config).unwrap()));

        let t1_engine = engine.clone();
        let t1 = thread::spawn(move || {
            let mut guard = t1_engine.lock().unwrap();
            guard.schedule_local_input(0);
        });

        let t2_engine = engine.clone();
        let t2 = thread::spawn(move || {
            let mut guard = t2_engine.lock().unwrap();
            guard.ingest_remote_input(0, 1);
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let mut guard = engine.lock().unwrap();
        let mut core = nes_core::NesCore::new();
        let _ = guard.advance_frame(&mut core);
    });

    #[cfg(not(loom))]
    {
        // Just run it normally to satisfy normal cargo test
        let config = RollbackConfig {
            local_player: 1,
            input_delay_frames: 0,
            max_rollback_frames: 60,
        };
        let engine = Arc::new(Mutex::new(RollbackEngine::new(config).unwrap()));

        let t1_engine = engine.clone();
        let t1 = thread::spawn(move || {
            let mut guard = t1_engine.lock().unwrap();
            guard.schedule_local_input(0);
        });

        let t2_engine = engine.clone();
        let t2 = thread::spawn(move || {
            let mut guard = t2_engine.lock().unwrap();
            guard.ingest_remote_input(0, 1);
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let mut guard = engine.lock().unwrap();
        let mut core = nes_core::NesCore::new();
        let _ = guard.advance_frame(&mut core);
    }
}
