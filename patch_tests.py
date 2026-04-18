import re

def main():
    with open('crates/nes-desktop/src/netplay.rs', 'r') as f:
        content = f.read()

    test = """#[test]
    fn poll_netplay_client_and_advance_frame_handles_missing_client_gracefully() {
        let mut core = nes_core::NesCore::default();
        let mut engine = nes_netplay::RollbackEngine::new(0);
        let mut now = std::time::Instant::now();
        let mut next_ping = now;
        let mut nonce = 0;
        let mut pings = std::collections::BTreeMap::new();
        let mut stats = None;
        let mut ctx = NetplayContext {
            netplay_client: None,
            rollback_engine: &mut engine,
            core: &mut core,
            netplay_local_player: 1,
            netplay_stats: &mut stats,
            now,
            netplay_next_ping_at: &mut next_ping,
            netplay_ping_nonce: &mut nonce,
            netplay_pending_pings: &mut pings,
            netplay_hash_check_every: 0,
        };
        let res = super::poll_netplay_client_and_advance_frame(&mut ctx);
        assert!(res.is_ok());
    }"""

    # insert before the last }
    idx = content.rfind("}")
    if idx != -1:
        content = content[:idx] + test + "\n" + content[idx:]

    with open('crates/nes-desktop/src/netplay.rs', 'w') as f:
        f.write(content)

if __name__ == '__main__':
    main()
