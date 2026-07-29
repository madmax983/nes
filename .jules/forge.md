**[Refactoring redundant controller input updates in nes-core api.rs]
**Learning:** Found massive boilerplate in the `execute` method of `api.rs` where every controller state modification command manually called `self.ports.set_controller_bits` and `self.sync_ppu_register_image()`. This increased risk of forgetting the sync call for future input commands.
**Action:** Flattened the execution block using early returns inside the `match` expression for non-input commands, letting input commands evaluate to a `(player, bits)` tuple. The actual state modification and synchronization is now performed exactly once at the bottom of the function.**Context Struct Extraction for Action Dispatchers**
**Learning:** Functions like `dispatch_app_action` and `dispatch_overlay_command` had grown to take 17 arguments, requiring `#[allow(clippy::too_many_arguments)]`.
**Action:** Grouped all these references into a single `struct AppContext<'a>` and passed `ctx: &mut AppContext<'_>` instead. This dramatically reduces signature size, makes the code much more readable, and allows removing the clippy suppression.

**Refactoring RTA Manager State Machine and I/O logic in nes-desktop/src/rta.rs**
**Learning:** Found two "God Functions" in the `RtaManager` (`tick` and `write_artifacts_if_finished`). `tick` was a 70+ line method with multiple levels of nesting and sequential state machine steps. `write_artifacts_if_finished` was a monolithic 60+ line method handling both JSON serialization and file writing for multiple artifacts. Additionally, the `select_profile` method used a confusing `.expect()` and `.next()` iterator pattern for filtering.
**Action:** Flattened `select_profile` by capturing `next()` using `let Some(...) else { return }`. Split the `tick` method into `tick_start`, `tick_pause_resume`, `tick_splits`, and `tick_end`. Split `write_artifacts_if_finished` into `write_run_artifact` and `write_input_log`. This drastically improves readability and reduces nesting.

**[Refactoring repetitive parsing logic]**
**Learning:** Argument parsing routines with many options often devolve into a "pyramid of doom" consisting of nearly identical `if arg == ...` and `if let Some(value) = arg.strip_prefix(...)` blocks, causing unnecessary scrolling and boilerplate.
**Action:** Extract a helper function (e.g., `parse_arg`) that abstracts the common shape of checking for both exact matches (e.g. `--flag`) and prefix matches (e.g. `--flag=value`), taking a closure to apply the specific field mutations cleanly.
**Extract RtaProfile report logic**\n**Learning:** The `write_draft_profile` method on `CalibrationRecorder` was a God Function at ~90 lines long, handling profile generation, file system creation, TOML serialization, report generation, and JSON serialization. \n**Action:** Extracted the core profile building logic into a pure `build_draft_profile` helper and moved the report I/O into `write_draft_report`, flattening the execution flow. When extracting array logic to helpers, ensuring functions take owned parameters like `Vec<T>` avoids unnecessary cloning when the caller was already consuming an owned collection.

**Extracting Background Palette Tile Cache Logic**
**Learning:** Found a large, deep block of logic inside `background_palette_index_cached` (in `crates/nes-core/src/ppu.rs`) spanning 16+ lines dedicated solely to reading PPU memory and updating a 4-color palette cache on a cache miss. This bloated the core rendering pipeline.
**Action:** Extracted this entire cache-miss population block into a standalone `populate_bg_tile_cache` helper function that takes the `BgTileCacheKey`. This flattened the hot path conditional from 16 lines down to a single function call, improving readability while strictly preserving zero-behavior-change semantics.
**[Extract profile name formatting]
**Learning:** In `crates/nes-desktop/src/rta.rs`, `select_profile` had duplicate manual `for` loops to concatenate a comma-separated list of profile IDs without allocating a standard vector buffer. This bloated the error handling blocks.
**Action:** Extracted the manual join loop into a pure generic `format_profile_names<'a>(profiles: impl Iterator<Item = &'a LoadedProfile>) -> String` helper function, preserving the zero-allocation characteristics while drastically improving readability and DRY-ness across both matched error conditions.
**[Refactoring excessive unwrap() usage in trainer loop]**
**Learning:** Found multiple usages of `model.as_ref().unwrap()` and reassignment via `.take().expect(...)` inside the hot PPO update loop in `nes-ai/src/trainer.rs`. This added visual clutter and made ownership unclear.
**Action:** Changed the signature of `ppo_update` to take `mut model: HybridPolicyValueNet` and return it back to the caller instead of taking a mutable reference to an `Option`. This eliminated all `unwrap()` calls on the model in both the outer loop and the inner update function, greatly improving clarity and explicitly modeling the ownership transfer through the optimizer mapping step.

**[Refactoring select_profile to flatten matches and use early returns]
**Learning:** Found a nested loop and match pattern in `nes-desktop/src/rta.rs` where `select_profile` was handling profile filtering. It used `if let Some(first) = ...` followed by `if let Some(second) = ...` without early returning from errors smoothly.
**Action:** Flattened the execution flow of `select_profile` by extracting the draft rule logic to a closure `check_draft` and using early returns via `if let Some(...) else { return ... }`.

**[Refactoring print_metrics_table]
**Learning:** Found a lot of repeated code with `table.add_row(vec![Cell::new("key"), Cell::new(val)])`.
**Action:** Created an inline helper closure `add_row` to remove the `.add_row(vec![...])` boilerplate, increasing DRY-ness.
**Use Safe Error Handling in Tests**\n**Learning:** Bare `.unwrap()` calls in tests can lead to opaque panics that hide the root cause. Using `.expect()` with a descriptive message is preferred.\n**Action:** Replaced `unwrap()` with `expect("valid config")` in `crates/nes-netplay/src/rollback.rs`.
**[Flattening deeply nested option unwrapping via Guard Clauses]
**Learning:** Functions like `parse_expr` and `handle_load_state` used cascading `if let Some() { ... } else { ... }` blocks that indented the happy path. This causes 'Pyramid of Doom' readability smells.
**Action:** Use guard clauses (`let Some(x) = y else { return ... };`) to flatten the logic so the successful execution path stays un-indented at the function root.
**[Extract encode_ppm into nes-core/ppm]
**Learning:** Found duplicate implementations of `encode_ppm` across `nes-desktop` and `nes-mcp`. This logic is not directly tied to those layers but is a core utility that should be tested and reused across the system. It lacked sufficient validation against bounds and panics.
**Action:** Extracted `encode_ppm` to a standalone, fully-tested `nes_core::ppm` module, removing duplication across crates. Replaced the `unwrap()` in size calculations with checked arithmetic to return proper IO errors instead of panicking on DoS inputs.
**[Refactoring unwrap in nes-dsl/src/parser.rs]**
**Learning:** Found an `is_empty()` check coupled with a subsequent `.unwrap()` on the same structure. This is considered an anti-pattern as it couples two lines when one operation could do both and leaves the `unwrap` lying around.
**Action:** Replace manual `.is_empty()` checks paired with `.unwrap()` on collections with `if let` or `let Some(...) = ... else { ... }` guard clauses to safely unwrap elements and improve readability.

**[Refactoring unwrap_or(Value::Null) in nes-desktop/src/mcp_host.rs]**
**Learning:** Found an instance of `unwrap_or(Value::Null)`. This violates clippy and idiomatic Rust guidelines because `Value::Null` is the default for `serde_json::Value`.
**Action:** Use `.unwrap_or_default()` instead of explicitly providing the default value like `.unwrap_or(serde_json::Value::Null)`.
**[Extract reset ephemeral state]
**Learning:** Found duplicate logic in `crates/nes-desktop/src/main.rs` for `AppAction::OpenRom`, `AppAction::LoadSlot`, and `AppAction::Reset` to reset the time machine, rewind state, audio, and metrics.
**Action:** Extracted this into `reset_ephemeral_state` taking `&mut AppContext` and reducing repetitive logic and boilerplate.

**[Flattening deeply nested option unwrapping via Guard Clauses in classify_keyboard_input]**
**Learning:** Functions like `classify_keyboard_input` used cascading `if let Some() { ... } else if let Some() { ... } else { ... }` blocks that indented the happy path. This causes 'Pyramid of Doom' readability smells.
**Action:** Use guard clauses (`let Some(x) = y else { return ... };`) to flatten the logic so the successful execution path stays un-indented at the function root.

**[Extract repetitive match logic to macro]**
**Learning:** Found massive boilerplate in the `apply_delta` method of `api.rs` where a giant `match` block matched on every variant of `MapperDeltaKind` only to extract the mapped value and apply state in identical ways.
**Action:** Created local macros `restore_copy!` and `restore_clone!` to drastically reduce the size of the match block, removing the "Pyramid of Doom" and increasing readability.

**[Flatten apply_delta using match tuple]**
**Learning:** Found that even with macros, we can improve readability by matching on `(&mut *self, &delta.kind)` together as a tuple, which completely removes the need for macros and nested `if let` chains in `apply_delta`.
**Action:** Replaced the macro approach with a single grouped tuple pattern match to further flatten logic and improve Codecov coverage.
