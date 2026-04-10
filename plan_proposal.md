1. Use `write_file` to create a new chaos test `crates/nes-desktop/tests/havoc.rs` that explicitly allocates `usize::MAX` in a memory buffer. This proves that an unchecked `.unwrap()` when writing to memory vectors can cause unrecoverable Out of Memory (OOM) aborts. The test will be annotated with `#[ignore]` to adhere to CI requirements.
2. Use `run_in_bash_session` to verify the new test file exists and contains the expected contents (`cat crates/nes-desktop/tests/havoc.rs`).
3. Use `run_in_bash_session` to run `cargo test --workspace` to ensure all tests pass (the new chaos test will be ignored due to the `#[ignore]` tag, but we can verify compilation).
4. Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
5. Submit the change with a title starting with '👺 Havoc: ' and a description detailing the trigger, stack trace, reproduction, and comment.
