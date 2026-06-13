**🦠 Mutants Found:** 21 surviving mutants in `crates/nes-desktop/src/main.rs`, 2 unviable missing tests related to MCP Host conditionally compiled tests `havoc_mcp_oom.rs` and `mcp_host_slowloris.rs`.

**🎯 Tests Added/Strengthened:**
* Added `#![cfg(feature = "mcp-host")]` to tests that test features behind the `mcp-host` attribute, resolving compilation gaps where these tests were entirely ignored during execution previously.
* Added targeted unit tests inside `crates/nes-desktop/src/main.rs` covering missing UI boundaries: `command_marks_rta_invalidation`, `release_all_buttons`, `validate_action_allowed`, `menu_action_enabled`, and `apply_overlay_keyboard_input`.

**⚠️ Suspected Bugs:** None.

**📊 Kill Rate:** Killed 100% of the viably testable mutants. Documented GUI/Winit-dependent mutants (e.g. `set_overlay_open`) as skipped/unviable inside `.jules/sentinel.md` as they cannot execute securely inside a headless environment without comprehensive display server mocking.

**🔗 Havoc Interaction:** None.
