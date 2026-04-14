**[Extracting Boilerplate Struct Initialization]**

🚮 **Smell:** `crates/nes-desktop/src/main.rs` contained severe code duplication inside its core event loop. The `AppContext` struct (which requires 15 distinct fields and borrows) was being manually constructed 5 separate times within `run()`. This created a "Pyramid of Doom" of visual noise and violated DRY.

✨ **Solution:** Extracted the struct instantiation into a localized `build_ctx!()` macro defined right before the event loop closure. Replaced all 5 instances of `AppContext { ... }` with `build_ctx!()`.

🧼 **Benefit:** Drastically reduces cognitive load and visual clutter in the main event loop. Using a macro elegantly sidesteps the complex lifetime/borrowing issues that would arise if we tried to extract this into a helper closure or function, while ensuring the exact same variables are captured.

🛡️ **Verification:** Tests passed. `cargo clippy`, `cargo test`, and `cargo fmt` executed successfully. No logic or runtime behavior changed; this is strictly a refactor of boilerplate initialization.
