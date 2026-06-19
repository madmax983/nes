# 👺 Havoc: Mutex Poisoning DOS via publish closure Panic

🧨 **The Trigger:** Panic inside `publish_frame_with` or `publish_audio_with` closure panics leaving `OnceLock<Mutex<OutputState>>` poisoned forever, leading to panic on any other function accessing the output state like `frame_chunk` or `audio_chunk`.

📉 **The Stack Trace:**
The `expect("output state lock")` call on line 273 triggers a panic due to the poisoned mutex.

🧪 **Reproduction:** "Run `cargo test --test havoc_mcp_audio_dos`."

😈 **Comment:** "You assumed developers would never write buggy code inside your closure. You were wrong. One bad closure takes down the whole MCP daemon."
