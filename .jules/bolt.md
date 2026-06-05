**[Eliminate Vec::with_capacity on Hot Paths]**
**Learning:** `Vec::with_capacity(n)` guarantees an immediate heap allocation. Using it on a hot path (like a per-frame game loop) for a collection that is empty 99.9% of the time actually introduces constant, unnecessary allocations, completely defeating its purpose as an optimization.
**Action:** Use `Vec::new()` for collections on hot paths that are rarely populated, because `Vec::new()` defers heap allocation until the first element is explicitly pushed.
