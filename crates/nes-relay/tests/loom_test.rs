use loom::sync::{Arc, Mutex};
use loom::thread;
use std::collections::HashMap;

// A simplified mock of nes-relay's RelayState and RoomState to test for deadlocks with Loom.
#[derive(Default)]
struct MockRelayState {
    rooms: HashMap<String, MockRoomState>,
}

#[derive(Default)]
struct MockRoomState {
    players: Vec<u8>,
}

fn mock_cleanup_client(state: &Arc<Mutex<MockRelayState>>, room: &str, player: u8) {
    let mut guard = state.lock().unwrap();
    if let Some(room_state) = guard.rooms.get_mut(room) {
        room_state.players.retain(|&p| p != player);
    }
}

#[test]
#[ignore = "Havoc Loom Concurrency Attack"]
fn havoc_test_loom_cleanup_client_deadlock() {
    loom::model(|| {
        let mut initial = MockRelayState::default();
        let mut room_state = MockRoomState::default();
        room_state.players.push(1);
        initial.rooms.insert("room".to_owned(), room_state);
        let state = Arc::new(Mutex::new(initial));

        let t1_state = state.clone();
        let t1 = thread::spawn(move || {
            mock_cleanup_client(&t1_state, "room", 1);
        });

        let t2_state = state.clone();
        let t2 = thread::spawn(move || {
            mock_cleanup_client(&t2_state, "room", 1);
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}
