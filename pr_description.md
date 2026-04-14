🕸️ Tangle: The `main.rs` file in `nes-desktop` was bloated and contained frame capture and encoding logic (`should_capture_frame`, `capture_path_for_frame`, `write_frame_ppm`, `encode_ppm`). This logic is distinct from the core UI event loop, mixing image encoding and file I/O with application orchestration.
📐 Blueprint: Extracted the frame capture logic into a dedicated `crates/nes-desktop/src/capture.rs` internal module. Updated `main.rs` to register the new module via `pub(crate) mod capture;` and import its contents.
🧱 Stability: Reduced coupling, separated frame capture concern from `main.rs`, improving modularity and file size.
🔭 Verification: Builds successfully, strict separation enforced. `cargo test` and `cargo clippy` pass.
