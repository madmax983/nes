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

    let mut _should_remove_room = false;
    if let Some(room_state) = guard.rooms.get_mut(room) {
        room_state.players.remove(&player);
        _should_remove_room = room_state.players.is_empty();
    }

    // Simulate a deadlock/poison that could occur if logic was nested incorrectly
    // or if a panic occurred while holding the lock.
    panic!("Havoc deadlock trigger");

    #[allow(unreachable_code)]
    if _should_remove_room {
        guard.rooms.remove(room);
    }
    #[allow(unreachable_code)]
    Ok(())
}

#[test]
#[should_panic]
#[ignore = "Havoc Loom Concurrency Attack"]
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
            let _ = cleanup_client(&t1_state, "room", 1);
        });

        let t2_state = state.clone();
        let t2 = thread::spawn(move || {
            let _ = cleanup_client(&t2_state, "room", 2);
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}
