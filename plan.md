1. **Havoc Target 1: `mcp_host` Slowloris Deadlock**
    - The MCP host's TCP listener thread reads requests synchronously. A malicious payload without proper termination causes a deadlock by blocking on `read_line` or reading massive payloads without returning.
    - Write a test `havoc_mcp_host_slowloris` that simulates this exact attack and assert that it stalls other clients. (Already verified in exploration).

2. **Havoc Target 2: `nes-mcp` output memory DOS (OOM)**
    - The `nes-mcp` crate has `publish_audio_with(len, ...)` and `publish_frame_with(width, height, ...)`.
    - They blindly allocate massive arrays in memory (`buffer.resize(len, 0)`).
    - Write a test `havoc_mcp_output_dos` that passes `1usize << 40` (1TB) to `publish_audio_with` to trigger an OOM crash. (Already verified in exploration).

3. **Havoc Target 3: WebRuntime Unsafe String Validation Panic**
    - The `nes-web` crate takes `&str` inputs from JS (`dispatch_dom_key`, `press_button`).
    - We will write a property-based test with arbitrary strings to prove we can crash or panic the WASM wrapper.
    - We'll write `havoc_web_runtime_input_fuzz`. (Already verified in exploration).

4. **Document the wreckage**
    - Compile findings into `.jules/havoc.md`.
    - Update PR with details matching Havoc persona.

5. **Pre-commit checks**
    - Run pre-commit checks to ensure code quality and stability.

6. **Submit PR**
    - Commit and submit.
