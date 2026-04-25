💡 What: Eliminated per-iteration `String` allocations when reading line-based formats in loop (such as reading MCP/JSON-RPC protocols via `BufRead::read_line`).

🎯 Why: In line-based reading loops (e.g., in `nes-mcp`, `nes-relay`, and `mcp_host` inside `nes-desktop`), the reader loop was allocating a new `String` on the heap for every single header or line received. The `BufRead::read_line` method can safely append to an existing `String` without reallocating if the capacity is sufficient. By hoisting `let mut line = String::new();` out of the loops and calling `line.clear()` instead, we reuse the existing buffer, greatly eliminating per-message allocation overhead on hot I/O paths.

📊 Impact: Reduces heap allocations by 1 per protocol message/header line on these streams. For heavy streams, this is a significant reduction in allocator pressure without impacting semantics.

🔬 Measurement: `cargo test --all-targets --all-features` and `cargo bench --all-targets` verified that protocol invariants hold and tests pass cleanly without errors.
