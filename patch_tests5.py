import re

def main():
    with open('crates/nes-desktop/src/netplay.rs', 'r') as f:
        content = f.read()

    old_test = """#[test]
    fn poll_netplay_client_and_advance_frame_handles_missing_client_gracefully() {"""

    new_test = """#[test]
    fn poll_netplay_client_and_advance_frame_handles_client_interaction() {
        let mut core = nes_core::NesCore::default();
        let mut engine = nes_netplay::RollbackEngine::new(nes_netplay::RollbackConfig {
            max_rollback_frames: 4,
            input_delay_frames: 0,
            local_player: 1,
        })
        .unwrap();
        let now = std::time::Instant::now();
        let mut next_ping = now;
        let mut nonce = 0;
        let mut pings = std::collections::BTreeMap::new();
        let mut stats = None;
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_tx2, rx) = std::sync::mpsc::channel();
        let (_tx3, err_rx) = std::sync::mpsc::channel();
        let client = super::NetplayClient { tx, rx, err_rx };
        let mut ctx = super::NetplayContext {
            netplay_client: Some(&client),
            rollback_engine: &mut engine,
            core: &mut core,
            netplay_local_player: 1,
            netplay_stats: &mut stats,
            now,
            netplay_next_ping_at: &mut next_ping,
            netplay_ping_nonce: &mut nonce,
            netplay_pending_pings: &mut pings,
            netplay_hash_check_every: 1,
        };
        let res = super::poll_netplay_client_and_advance_frame(&mut ctx);
        assert!(res.is_ok());
    }

    #[test]
    fn poll_netplay_client_and_advance_frame_handles_missing_client_gracefully() {"""

    content = content.replace(old_test, new_test)

    with open('crates/nes-desktop/src/netplay.rs', 'w') as f:
        f.write(content)

if __name__ == '__main__':
    main()
