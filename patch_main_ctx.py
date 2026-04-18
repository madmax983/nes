import re

def main():
    with open('crates/nes-desktop/src/main.rs', 'r') as f:
        content = f.read()

    old_call = """                if let Err(err) = crate::netplay::poll_netplay_client_and_advance_frame(
                    netplay_client.as_ref(),
                    rollback_engine,
                    &mut core,
                    netplay_local_player,
                    &mut netplay_stats,
                    now,
                    &mut netplay_next_ping_at,
                    &mut netplay_ping_nonce,
                    &mut netplay_pending_pings,
                    netplay_hash_check_every,
                )"""

    new_call = """                if let Err(err) = crate::netplay::poll_netplay_client_and_advance_frame(
                    &mut crate::netplay::NetplayContext {
                        netplay_client: netplay_client.as_ref(),
                        rollback_engine,
                        core: &mut core,
                        netplay_local_player,
                        netplay_stats: &mut netplay_stats,
                        now,
                        netplay_next_ping_at: &mut netplay_next_ping_at,
                        netplay_ping_nonce: &mut netplay_ping_nonce,
                        netplay_pending_pings: &mut netplay_pending_pings,
                        netplay_hash_check_every,
                    }
                )"""

    content = content.replace(old_call, new_call)

    with open('crates/nes-desktop/src/main.rs', 'w') as f:
        f.write(content)

if __name__ == '__main__':
    main()
