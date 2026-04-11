use std::io::Write;
use std::process::{Command, Stdio};

#[test]
#[ignore = "havoc target"]
fn havoc_crash_desktop_mcp_host_oom() {
    // We launch `nes-desktop` in headless/rta mode or something similar so it binds MCP but
    // doesn't block on a window if possible.
    // Wait, the desktop app ALWAYS opens a window unless we pass an RTA command that finishes...
    // Actually, `nes-mcp` is much easier to crash and it's already tested.
    // Let's focus on `nes-relay` and the string splitting in network packets.
}
