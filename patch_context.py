import re

def main():
    with open('crates/nes-desktop/src/netplay.rs', 'r') as f:
        content = f.read()

    context_struct = """pub struct NetplayContext<'a> {
    pub netplay_client: Option<&'a NetplayClient>,
    pub rollback_engine: &'a mut nes_netplay::RollbackEngine,
    pub core: &'a mut nes_core::NesCore,
    pub netplay_local_player: u8,
    pub netplay_stats: &'a mut Option<NetplayRuntimeStats>,
    pub now: std::time::Instant,
    pub netplay_next_ping_at: &'a mut std::time::Instant,
    pub netplay_ping_nonce: &'a mut u64,
    pub netplay_pending_pings: &'a mut std::collections::BTreeMap<u64, std::time::Instant>,
    pub netplay_hash_check_every: u64,
}

"""

    content = content.replace("#[allow(clippy::too_many_arguments)]\npub fn poll_netplay_client_and_advance_frame(\n    netplay_client: Option<&NetplayClient>,\n    rollback_engine: &mut nes_netplay::RollbackEngine,\n    core: &mut nes_core::NesCore,\n    netplay_local_player: u8,\n    netplay_stats: &mut Option<NetplayRuntimeStats>,\n    now: std::time::Instant,\n    netplay_next_ping_at: &mut std::time::Instant,\n    netplay_ping_nonce: &mut u64,\n    netplay_pending_pings: &mut std::collections::BTreeMap<u64, std::time::Instant>,\n    netplay_hash_check_every: u64,\n) -> Result<(), String> {", context_struct + "pub fn poll_netplay_client_and_advance_frame(ctx: &mut NetplayContext<'_>) -> Result<(), String> {")

    # Replace variables with ctx.*
    replacements = {
        "netplay_client": "ctx.netplay_client",
        "rollback_engine": "ctx.rollback_engine",
        "core": "ctx.core",
        "now": "ctx.now",
        "netplay_next_ping_at": "ctx.netplay_next_ping_at",
        "netplay_ping_nonce": "ctx.netplay_ping_nonce",
        "netplay_pending_pings": "ctx.netplay_pending_pings",
        "netplay_local_player": "ctx.netplay_local_player",
        "netplay_stats": "ctx.netplay_stats",
        "netplay_hash_check_every": "ctx.netplay_hash_check_every"
    }

    # We need to be careful with replacement, let's just do it manually for the function body

    body_start = content.find("pub fn poll_netplay_client_and_advance_frame")
    body_end = content.find("pub fn handle_netplay_server_message")

    body = content[body_start:body_end]

    for old, new in replacements.items():
        # Add word boundaries so we don't accidentally replace substrings
        body = re.sub(r'\b' + old + r'\b', new, body)

    content = content[:body_start] + body + content[body_end:]

    with open('crates/nes-desktop/src/netplay.rs', 'w') as f:
        f.write(content)

if __name__ == '__main__':
    main()
