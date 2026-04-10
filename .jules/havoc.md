**OOM in nes-desktop MCP Host**
**The Trigger:** A malicious MCP host connection sending an artificially large `Content-Length:` header (e.g., 18446744073709551615).
**The Stack Trace:** Panic at `vec![0_u8; len]` due to "capacity overflow" or system out-of-memory.
**Reproduction:** Run `cargo test -p nes-desktop --test havoc -- --ignored`.
**Comment:** You blindly allocated memory based on an unauthenticated client's header request. Never trust the network. I just DOS'd your emulator with 40 bytes.
