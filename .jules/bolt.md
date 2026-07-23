## 2025-02-18 - Eliminated MMC5 CHR Region Heap Allocations
**Learning:** Returning small variable-sized collections via `Vec` introduces significant overhead on hot paths, especially state restoration or mapper syncing, by hitting the allocator.
**Action:** Replace small `Vec` return types (up to N elements) with `([T; N], usize)` tuples. It drastically reduces heap pressure while cleanly integrating with iteration using slice bounds `&regions[..count]`.
