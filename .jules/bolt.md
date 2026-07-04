**Vec Allocation Optimization on Hot Paths**
**Learning:** `Vec::with_capacity(N)` guarantees an immediate heap allocation. On hot paths (like per-frame ticks) where a vector remains empty in the vast majority of cases, this is a false optimization that causes continuous allocate/deallocate churn.
**Action:** Use `Vec::new()` instead, which defers heap allocation entirely until the first element is pushed, dramatically reducing memory allocator pressure for rare events.
