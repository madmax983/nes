**🧬 Mutants Found:**
Count and summary: Found 206 mutants tested across `nes-test-harness` and `nes-config` in targeted runs.
- `nes-test-harness/src/lib.rs`: 178 mutants. Killed most. Identified 2 surviving logic gaps (now closed), 1 expected test weakness (now closed), and several equivalent bitwise mutants (documented).
- `nes-test-harness/src/homebrew.rs`: 8 mutants. 100% killed natively.
- `nes-test-harness/src/rom_paths.rs`: 16 mutants. Expected weaknesses natively due to `nes.toml` config dependencies for copyrighted ROM paths (documented).
- `nes-config/src/lib.rs`: Identified multiple timeout mutants in CLI loop parsing loops.

**🎯 Tests Added/Strengthened:**
- **`nes-test-harness/src/lib.rs`**: Strengthened `collect_apu_register_writes_tracks_apu_bus_stores` to assert `apu_write_hash` inequalities explicitly when components of the `ApuWriteEvent` are bitwise mutated.
- **`nes-test-harness/src/lib.rs`**: Added `collect_apu_register_writes_ignores_reads` and `collect_apu_register_writes_ignores_non_apu_writes` to kill mutants changing the collection conditional logic from `&&` to `||` or mutating the address check.
- **`nes-config/src/lib.rs`**: Added `parse_config_path_loop_terminates` to assert that internal parsing loops iterate exactly the expected amount and terminate, responding to timeout mutants found when mutating the `idx += 1` assignments.

**⚠️ Suspected Bugs:**
None. Surviving mutants were either weak assertions or expected equivalent/timeout mutants.

**📊 Kill Rate:**
Improved kill rates in `nes-test-harness` core functions by capturing bitwise mutations on `apu_write_hash` and `collect_apu_register_writes` logic boundaries. `nes-config` parsing logic is now firmly asserted to terminate correctly.

**🔗 Havoc Interaction:**
Equivalent mutants and timeouts logged to `.jules/sentinel.md` journal.
