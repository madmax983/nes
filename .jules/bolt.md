**[Bolt] Replace Vec with SmallVec on hot path**
**Learning:** The `RtaManager::tick` method is called every frame to evaluate triggers. While it pre-allocated `Vec::with_capacity(2)` to avoid re-allocations on growth, `Vec::with_capacity` still guarantees a heap allocation.
**Action:** Swapped `Vec::with_capacity(2)` for `smallvec::SmallVec::<[RtaEvent; 2]>::new()` on the hot path in `nes-desktop/src/rta.rs` to completely eliminate the per-frame heap allocation, keeping the events on the stack unless they exceed 2 items (which rarely happens).

**[Bolt] Replace Vec with SmallVec on hot path**
**Learning:** The `RtaManager::tick` method is called every frame to evaluate triggers. While it pre-allocated `Vec::with_capacity(2)` to avoid re-allocations on growth, `Vec::with_capacity` still guarantees a heap allocation. Actually, `Vec::new()` without capacity achieves the same 0-allocation goal because `Vec::new` does not allocate until the first push.
**Action:** Swapped `Vec::with_capacity(2)` for `Vec::new()` on the hot path in `nes-desktop/src/rta.rs` to completely eliminate the per-frame heap allocation.
