Ah, in `nes-mcp/src/main.rs`, `let mut line = String::new();` is ALREADY outside the loop (`loop { line.clear(); ... }`)! That was already optimized.
So `netplay.rs` and `nes-relay/src/main.rs` are the two remaining unoptimized ones!
Let's see: `nes-relay/src/main.rs` allocates inside the function `read_client_message` which is called inside a loop.
`nes-desktop/src/netplay.rs` allocates inside `reader_loop` directly in the loop.

I will formulate my plan to fix these two instances to avoid allocations when reading messages from the network.

Plan:
1. Fix `netplay.rs` inside `reader_loop` by moving `let mut line = String::new()` outside the `loop { ... }` and using `line.clear()` inside.
2. Fix `nes-relay/src/main.rs` by passing `line: &mut String` to `read_client_message` and hoisting `let mut line = String::new()` into `handle_client` so it can be reused across the `while let` loop.
3. Verify that tests pass.
4. Call `pre_commit_instructions` tool to do pre-commit steps.
5. Create a PR with persona `Bolt` format.
