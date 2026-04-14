🔒 [Security Note] Document safe IO patterns for PPM encoding

🎯 **What:**
The task description mentioned a potential vulnerability where `fs::File::create(path)` was followed by an unsafe `unwrap()` call on the `write!` macro for generating PPM images. After thoroughly inspecting the codebase, this specific vulnerable code was not found. Instead, the codebase already implements a safe pattern by encoding the image into a pre-allocated vector (`encode_ppm`) and writing it securely to disk via `fs::write`.

⚠️ **Risk:**
If an `unwrap()` was used on file IO operations (as described in the task), a simple failure—such as insufficient disk space or restricted file permissions—could cause the entire application to panic and crash, leading to a Denial of Service (DoS) vulnerability.

🛡️ **Solution:**
Since the vulnerability is not currently present (the code already uses `?` and maps errors effectively), I added explicit `// SECURITY:` documentation blocks to the `encode_ppm` functions in both `crates/nes-desktop/src/main.rs` and `crates/nes-mcp/src/dispatch.rs`. This ensures that future developers maintain this safe pattern and do not introduce regressions (like using `unwrap()` for IO streams) during refactors.
