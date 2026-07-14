**[Title]** nes-core Weak Assertion Fixes
**Mutant:** Unviable and uncaught mutants in `ppm.rs`, `serde_array.rs`, `bus.rs`, `cheat_codes.rs`, and `cpu/status.rs`.
**Diagnosis:** The existing tests did not properly verify exact outputs (e.g., `ppm.rs` padding/metadata correctness), allowed bitwise operator replacement in `cheat_codes.rs` (e.g. `^` instead of `&`), or didn't explicitly assert bit configurations for flag retrievals in `status.rs`.
**Kill Shot:** Implemented highly targeted, explicit assertion tests (`sentinel_*.rs`) covering precise edge-case behavior and binary mask parsing logic.
