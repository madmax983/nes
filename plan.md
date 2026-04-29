## Plan
1. Fix the Out-Of-Memory (OOM) attack vulnerability in `nes-mcp`'s `load_rom` tool caused by unbounded file reading.
   - Modify `parse_rom_payload` in `crates/nes-mcp/src/dispatch.rs` to read at most a specific max size (e.g. 5 MB).
2. Fix the OOM attack vulnerability in `nes-desktop`'s `load_state_file` caused by unbounded file reading.
   - Modify `read_save_state_file` in `crates/nes-desktop/src/manual_state.rs` to limit the file read size (e.g. 5 MB) instead of blindly using `fs::read`.
3. Fix the OOM attack vulnerability in `nes-desktop`'s `load_rom_session` caused by unbounded file reading.
   - Modify `load_rom_session` in `crates/nes-desktop/src/session.rs` to limit the file read size instead of blindly using `fs::read`.
4. Run the ignored havoc test `havoc_load_rom_oom` to verify the `nes-mcp` fix.
5. Run the ignored havoc test `havoc_desktop_load_rom_oom` to verify the `nes-desktop` `manual_state` fix.
6. Create an equivalent test for `load_rom_session` and verify that it no longer crashes.
7. Complete pre commit steps to make sure proper testing, verifications, reviews and reflections are done.
8. Submit the change using `submit` tool.
