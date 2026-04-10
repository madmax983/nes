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

    // Attempting to deadlock without panicking in thread destructors
    let _ = state.lock().unwrap();

    if should_remove_room {
        guard.rooms.remove(room);
    }
    Ok(())
}

#[test]
#[should_panic]
fn havoc_test_loom_cleanup_client_deadlock() {
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

        t1.join().unwrap();
    });
}
