1. The Codecov CI failure indicates that when I moved `capture_config_from_parts`, `capture_path_for_frame`, and `netplay_feature_enabled` from `main.rs` to `config.rs`, I removed their respective unit tests from `main.rs` but did not re-add them to `config.rs`. This dropped the patch coverage below the required threshold.
2. I need to restore the removed unit tests to `crates/nes-desktop/src/config.rs` inside a `#[cfg(test)] mod tests` block.
3. Then I will run `cargo fmt` and `cargo test` to ensure it works.
