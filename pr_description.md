💡 What: Hoisted the `String::new()` buffer allocation outside the continuous `reader_loop` inside `crates/nes-desktop/src/netplay.rs` and replaced it with `line.clear()` inside the loop.

🎯 Why: To eliminate an unnecessary heap allocation per line when reading incoming payload messages over the network during an active netplay session. Calling `.clear()` allows the string to retain its pre-allocated capacity across loop iterations.

📊 Impact: Reduces heap allocations by 1 for every received netplay message (60 times a second per active connection).

🔬 Measurement: Compile and observe identical functionality during netplay tests (`cargo test -p nes-desktop`). Run performance profiling against a live session.
