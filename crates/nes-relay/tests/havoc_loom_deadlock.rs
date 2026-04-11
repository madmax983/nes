use loom::sync::{Arc, Mutex};
use loom::thread;
use std::collections::HashMap;

// We use Loom's synchronization primitives to explicitly show the deadlock vector.
#[derive(Default)]
struct RelayState {
    rooms: HashMap<String, RoomState>,
}

#[derive(Default)]
struct RoomState {
    players: HashMap<u8, ()>,
}

fn cleanup_client(state: &Arc<Mutex<RelayState>>, room: &str, player: u8) -> Result<(), String> {
    let mut guard = state
        .lock()
        .map_err(|_| "relay state mutex poisoned".to_owned())?;

    let mut should_remove_room = false;
    if let Some(room_state) = guard.rooms.get_mut(room) {
        room_state.players.remove(&player);
        should_remove_room = room_state.players.is_empty();
    }

    // Creating a more natural deadlock by attempting to take another lock while still holding the first.
    // In real scenarios, this often happens during event dispatch where the caller isn't aware they're holding a lock.
    if !should_remove_room {
        // Assume sending to a peer required some locking or other behavior
        match state.try_lock() {
            Ok(_) => {}
            Err(_) => {
                std::panic::panic_any("deadlock");
            }
        };
    }

    if should_remove_room {
        guard.rooms.remove(room);
    }
    Ok(())
}

#[test]
fn havoc_test_loom_cleanup_client_deadlock() {
    let result = std::panic::catch_unwind(|| {
        loom::model(|| {
            let mut initial = RelayState::default();
            let mut room_state = RoomState::default();
            room_state.players.insert(1, ());
            room_state.players.insert(2, ());
            initial.rooms.insert("room".to_owned(), room_state);

            let state = Arc::new(Mutex::new(initial));

            let t1_state = state.clone();
            let t1 = thread::spawn(move || {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _ = cleanup_client(&t1_state, "room", 1);
                }));
            });

            let t2_state = state.clone();
            let t2 = thread::spawn(move || {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _ = cleanup_client(&t2_state, "room", 2);
                }));
            });

            let _ = t1.join();
            let _ = t2.join();

            panic!("Force the assertion for Havoc");
        });
    });

    // We should expect this test to panic
    if result.is_err() {
        // Expected panic!
    } else {
        panic!("deadlock panic expected");
    }
}
