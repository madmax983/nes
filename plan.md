As Havoc, my job is to prove fragility in the system. I will write tests to break it but not fix the actual bugs. I found a new fragility:

1. **OOM Vulnerability in `nes-mcp::dispatch_tool` (load_rom)**:
   - When the user calls the `load_rom` tool with `rom_path: "/dev/zero"`, `std::fs::read` attempts to read the entire file into a buffer. Since `/dev/zero` is an infinite stream, this allocates memory continuously until the process runs out of memory and gets killed by the OS (SIGKILL).
   - I wrote the test `havoc_load_rom_oom.rs` to reproduce this crash.
   - I have appended this finding to `.jules/havoc.md`.

Next steps:
- Ensure the newly created test file and the journal entry are complete.
- Complete pre-commit instructions.
- Submit the changes with a PR named `👺 Havoc: [MCP Load ROM OOM]`
