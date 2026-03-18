**[Avoiding heap allocation in string comparisons]**
**Learning:** `to_uppercase()` creates a newly allocated `String`, which incurs a heap allocation overhead. In hot paths or parsers (like macro engine command parsing or controller button parsing), this allocation is unnecessary.
**Action:** Use `.eq_ignore_ascii_case()` on `&str` directly instead of allocating a new string via `.to_uppercase()`. This achieves the same result with zero allocations.

**[Avoiding heap allocation in string comparisons]**
**Learning:** `to_uppercase()` creates a newly allocated `String`, which incurs a heap allocation overhead. In hot paths or parsers (like macro engine command parsing or controller button parsing), this allocation is unnecessary.
**Action:** Use `.eq_ignore_ascii_case()` on `&str` directly instead of allocating a new string via `.to_uppercase()`. This achieves the same result with zero allocations.

**[Avoiding heap allocation in string comparisons]**
**Learning:** `to_uppercase()` creates a newly allocated `String`, which incurs a heap allocation overhead. In hot paths or parsers (like macro engine command parsing or controller button parsing), this allocation is unnecessary.
**Action:** Use `.eq_ignore_ascii_case()` on `&str` directly instead of allocating a new string via `.to_uppercase()`. This achieves the same result with zero allocations.

**[Avoiding heap allocation in string comparisons]**
**Learning:** `to_uppercase()` creates a newly allocated `String`, which incurs a heap allocation overhead. In hot paths or parsers (like macro engine command parsing or controller button parsing), this allocation is unnecessary.
**Action:** Use `.eq_ignore_ascii_case()` on `&str` directly instead of allocating a new string via `.to_uppercase()`. This achieves the same result with zero allocations.
**[Arc::make_mut for zero-copy IPC]**
**Learning:** `Arc::make_mut(&mut arc_vec)` gives a mutable reference to the inner `Vec` without allocating if the strong count is exactly 1 (e.g., zero active readers).
**Action:** Use `Arc::make_mut` paired with closures (e.g., `_with<F>`) to eliminate recurring huge heap allocations on hot paths like video rendering and audio streaming.
