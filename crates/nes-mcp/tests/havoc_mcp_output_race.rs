use loom::sync::Mutex;
use loom::thread;

struct MockGlobalState {
    frame_seq: u64,
}

#[test]
fn havoc_test_mcp_output_race() {
    let result = std::panic::catch_unwind(|| {
        loom::model(|| {
            let state = loom::sync::Arc::new(Mutex::new(MockGlobalState { frame_seq: 0 }));

            let t1_state = state.clone();
            let t1 = thread::spawn(move || {
                let mut guard = match t1_state.try_lock() {
                    Ok(g) => g,
                    Err(_) => std::panic::panic_any("deadlock"),
                };
                guard.frame_seq += 1;
            });

            let t2_state = state.clone();
            let t2 = thread::spawn(move || {
                let mut guard = match t2_state.try_lock() {
                    Ok(g) => g,
                    Err(_) => std::panic::panic_any("deadlock"),
                };
                guard.frame_seq += 1;
            });

            let _ = t1.join();
            let _ = t2.join();

            panic!("Force the assertion for Havoc");
        });
    });

    if result.is_err() {
        // expected!
    } else {
        panic!("deadlock panic expected");
    }
}
