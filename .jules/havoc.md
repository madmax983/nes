**👺 Havoc: [Thread Exhaustion DoS in Relay]**
🧨 **The Trigger:** Sending a flood of Input messages (e.g. 100,000/sec) when `latency-ms` is > 0 causes `nes-relay` to spawn an unbounded number of OS threads to delay forwarding the packets.
📉 **The Stack Trace:** (OS hang/OOM due to Thread exhaustion)
🧪 **Reproduction:** Run `cargo test havoc_crash_relay_thread_bomb -- --ignored`
😈 **Comment:** "You assumed OS threads were free and infinite. They are not. A single client can take down your entire netplay server."

**👺 Havoc: [Header Memory Exhaustion DoS in MCP]**
🧨 **The Trigger:** Sending an infinite string without `\n` to `nes-mcp`'s stdio reader causes `reader.read_line` to allocate unbounded memory until the system crashes.
📉 **The Stack Trace:** (OOM panic in alloc::raw_vec)
🧪 **Reproduction:** Pipe `/dev/zero` or `yes` to the stdin of `nes-mcp`.
😈 **Comment:** "You assumed network buffers would eventually find a newline. You were wrong."
