**[Vectors capacity]**
**Learning:** Pre-allocating vectors via `Vec::with_capacity(64)` for temporary, cleared-and-swapped buffers (`writes`, `mmio_reads`, etc. in `nes-core`) correctly minimizes heap allocations and reduces latency spikes during hot-path execution.
**Action:** Use `Vec::with_capacity(n)` instead of `Vec::new()` for frequently used buffers to achieve zero-cost abstraction efficiency without fighting the borrow checker.
