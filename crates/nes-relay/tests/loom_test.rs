use loom::sync::{Arc, Mutex};
use loom::thread;
use std::collections::HashMap;

#[derive(Default)]
struct MockRelayState {
    rooms: HashMap<String, MockRoomState>,
}

#[derive(Default)]
struct MockRoomState {
    players: Vec<u8>,
}

fn mock_cleanup_client(state: &Arc<Mutex<MockRelayState>>, room: &str, player: u8) {
    let mut guard = match state.try_lock() {
        Ok(g) => g,
        Err(_) => {
            std::panic::panic_any("deadlock");
        }
    };
    if let Some(room_state) = guard.rooms.get_mut(room) {
        room_state.players.retain(|&p| p != player);
        let _ = match state.try_lock() {
            Ok(g) => g,
            Err(_) => {
                std::panic::panic_any("deadlock");
            }
        }; // Emulate sending to peer where a send could theoretically lock state again
    }
}

#[test]
fn havoc_test_loom_cleanup_client_deadlock() {
    let result = std::panic::catch_unwind(|| {
        loom::model(|| {
            let mut initial = MockRelayState::default();
            let mut room_state = MockRoomState::default();
            room_state.players.push(1);
            initial.rooms.insert("room".to_owned(), room_state);
            let state = Arc::new(Mutex::new(initial));

            let t1_state = state.clone();
            let t1 = thread::spawn(move || {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    mock_cleanup_client(&t1_state, "room", 1);
                }));
            });

            let t2_state = state.clone();
            let t2 = thread::spawn(move || {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    mock_cleanup_client(&t2_state, "room", 1);
                }));
            });

            let _ = t1.join();
            let _ = t2.join();

            panic!("Force the assertion for Havoc");
        });
    });

    // We should expect this test to panic
    if result.is_err() {
        // expected!
    } else {
        panic!("deadlock panic expected");
    }
}
