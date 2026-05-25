1. Update journal in `.jules/sentinel.md` with the critical learnings from `constants.rs` timeouts.
   - I will append the following exact text using a Heredoc:
     ```
     **Timeout on Arithmetic Mutation**
     **Mutant:** Replaced `/` with `*` in AUDIO_CHUNK_SAMPLES computation (`crates/nes-core/src/constants.rs:21:69`)
     **Diagnosis:** The mutation changes the sample rate from a reasonable chunk size to a massive value (44100 * 60 = 2646000). This causes massive allocations in audio-related tests, leading to suite hangs and timeout mutants rather than explicit test failures.
     **Kill Shot:** Added an explicit test in `tests/sentinel_constants.rs` verifying `AUDIO_CHUNK_SAMPLES == 735` to immediately fail the test before downstream systems hang.
     ```
2. Verify journal update.
   - I will execute `cat .jules/sentinel.md` to verify the entry was appended properly.
3. Run linters and tests.
   - I will execute `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo fmt --all`.
4. Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
5. Submit the change.
   - I will submit the PR with branch `sentinel-core` and PR title `🤖 Sentinel: Strengthen test suite for nes-core bus and constants`. The description will follow Sentinel format.
