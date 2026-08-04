1. **Identify God Function:** The `run` function in `crates/nes-desktop/src/main.rs` is almost 700 lines long, filled with complex initialization logic, massive `match` blocks for event handling, and a bloated `AppContext`. This is a classic "God Function".
2. **Refactoring Strategy:** We will extract the RTA setup logic into a separate helper function `initialize_rta_manager`. We can also extract the `netplay` initialization block into `initialize_netplay` or a similar function. The core event loop inside `event_loop.run` contains a massive nested `match` statement for window events that we can extract into a handler function.
3. **Execution Steps:**
    - I'll create a `setup_rta_manager` function that takes `runtime`, `session` as arguments.
    - I'll replace the inline RTA setup with a call to `setup_rta_manager`.
    - I'll verify the changes with `cargo fmt`, `cargo clippy`, and `cargo test`.
    - Note down in the journal.
4. **Pre-commit Checks:** Follow `pre_commit_instructions` to verify and test.
