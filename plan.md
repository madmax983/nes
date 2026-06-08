1. **Optimize `Vec::new()` to `Vec::with_capacity()` in `nes-core/src/api.rs` and `nes-core/src/cpu/engine.rs`**
   - The memory arrays like `writes`, `prg_writes`, `mmio_reads`, and `bus_trace` are used as temporary buffers inside hot execution loops and are continuously swapped and cleared via `std::mem::swap`.
   - In `Cpu::new` (in `crates/nes-core/src/cpu/engine.rs`) and `NesCore::new` (in `crates/nes-core/src/api.rs`), these vectors are instantiated using `Vec::new()`, which allocates 0 capacity initially.
   - When the emulation starts, and for the first few instructions, pushing to these vectors incurs multiple reallocations on the heap until the vectors naturally grow to their steady-state capacity.
   - The optimization is to change these `Vec::new()` calls to `Vec::with_capacity(N)` in the constructors, avoiding the upfront allocations and keeping memory allocations strictly zero at runtime.
   - We will use `Vec::with_capacity(4)` or similar small numbers for the writes/reads since an instruction typically does not perform many writes/reads.

2. **Verify Performance Improvement**
   - Run tests to make sure there are no regressions.
   - Ensure the code passes `cargo clippy`, `cargo fmt`, and `cargo test`.
   - Add journal entries if needed.

3. **Complete pre-commit steps**
   - Run the pre commit instructions and check logic.

4. **Submit PR**
   - Submit PR with title "⚡ Bolt: [Zero-cost allocation optimization for Cpu buffers]"
