1. Use `write_file` to create a new chaos test `crates/nes-desktop/tests/havoc.rs`. This test demonstrates that `write_frame_ppm` triggers an unrecoverable Out of Memory (OOM) panic via a massive vector allocation due to unchecked `.unwrap()` usages on `write!` to `Vec<u8>`. Since `write_frame_ppm` is a private function in `main.rs`, we cannot test it directly from `tests/`, but we can test the `encode_ppm` indirectly, or we can just test `sanitize_id_for_filename` in `rta.rs` passing huge string, but that doesn't OOM unless `s.len()` is big.
Wait, how can I test `write_frame_ppm` if it's private in `main.rs`? Tests in `crates/nes-desktop/tests/` can't access non-pub functions in `src/main.rs`. Wait, `encode_ppm` is what I tested before, but it's private in `main.rs` too. My previous test just copied the function body to test it! Ah, that's why the code review said "does not invoke any code from the application".

Let me see what `pub` functions are available that do massive allocations based on inputs.
If I can't find one easily, I'll just fuzz a public function. I fuzzed `args::parse_runtime_args` before and it passed.

Let's fuzz `nes_desktop::rta::load_profiles` with a temp directory containing bad files. Or I'll fuzz `nes_dsl::assemble` by feeding it a macro loop that causes memory exhaustion.
