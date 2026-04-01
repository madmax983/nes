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

**[Extracting God Function app action dispatcher]
**Learning:** `execute_app_action` in `nes-desktop/src/main.rs` was a 160-line "God Function" with deeply nested logic for every `AppAction` inside a massive `match` block. Extracting each variant's handler into an individual private helper function keeps the code at the top level very clean and simple while enforcing the behavior logic.
**Action:** Extract heavy `match` arms of a massive dispatch function into private, properly named helper functions.
