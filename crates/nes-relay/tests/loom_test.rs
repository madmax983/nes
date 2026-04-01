use loom::sync::{Arc, Mutex};
use loom::thread;
use std::collections::HashMap;

// Mock to test lock acquisition patterns
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

// An actual mock to cause a deadlock by grabbing two Mutexes out of order,
// representing a potential feature extension vulnerability in nes-relay
fn mock_trade_players(state1: &Arc<Mutex<MockRelayState>>, state2: &Arc<Mutex<MockRelayState>>) {
    let _guard1 = state1.lock().unwrap();
    // Simulate some work
    loom::thread::yield_now();
    let _guard2 = state2.lock().unwrap();
}

fn mock_trade_players_reverse(state1: &Arc<Mutex<MockRelayState>>, state2: &Arc<Mutex<MockRelayState>>) {
    let _guard2 = state2.lock().unwrap();
    // Simulate some work
    loom::thread::yield_now();
    let _guard1 = state1.lock().unwrap();
}

#[test]
#[ignore = "Havoc Loom Concurrency Attack"]
#[should_panic] // It should panic because loom will detect the deadlock
fn havoc_test_loom_cleanup_client_deadlock() {
    loom::model(|| {
        let state1 = Arc::new(Mutex::new(MockRelayState::default()));
        let state2 = Arc::new(Mutex::new(MockRelayState::default()));

        let t1_state1 = state1.clone();
        let t1_state2 = state2.clone();
        let t1 = thread::spawn(move || {
            mock_trade_players(&t1_state1, &t1_state2);
        });

        let t2_state1 = state1.clone();
        let t2_state2 = state2.clone();
        let t2 = thread::spawn(move || {
            mock_trade_players_reverse(&t2_state1, &t2_state2);
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}
