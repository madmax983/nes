## 2023-10-24 - [Avoid unwrap_or clone on Option borrowed from HashMap]
**Learning:** [When dealing with an `Option<&Vec<T>>` returned from `HashMap::get`, using `.unwrap_or(&Vec::new()).clone()` forces an unnecessary heap allocation. Furthermore, `Vec` does not implement `map_or` directly, but `Option` does. Using `Option::is_none_or()` is the most idiomatic and performant way to evaluate a condition against the optional borrowed value without allocating.]
**Action:** [Always use `is_none_or` or `map_or` for conditional checks on borrowed `Option` values instead of cloning them or providing fallback heap allocations.]
**[Local variable doc comments]
**Learning:** [In Rust, applying a documentation comment (`///`) to a local statement (such as a `let` binding inside a function body) causes a compilation error (`error: expected outer doc comment`). Use standard comments (`//`) to document local variables and internal function logic.]
**Action:** [Always use `//` for comments inside functions, and reserve `///` for outer item declarations like functions, structs, and fields.]
**[Eliminating Header Buffer Allocations]
**Learning:** [When parsing line-based protocols (like MCP/JSON-RPC over stdio or TCP streams) in a loop, declaring `let mut line = String::new();` inside the loop causes a heap allocation for every single header line read.]
**Action:** [Hoist the `String` allocation outside the reading loop and call `line.clear()` before each `.read_line()` call. This allows `read_line` to safely reuse the existing buffer capacity, eliminating per-header-line allocations entirely.]
**[Optimized RtaManager Allocations]**
**Learning:** `TriggerRule` and `RtaProfile` struct members were unnecessarily cloned during `RtaManager` initialization on the hot path, causing heap allocations.
**Action:** Used `std::mem::take` to pass ownership of properties out of the `mut profile` instead of cloning them. Iterated over `profile.splits.iter_mut()` instead of `.iter()`.
**[Optimized RtaManager Allocations]**
**Learning:** `TriggerRule` and `RtaProfile` struct members are owned by `RtaManager` and cannot be `take`n without leaving empty rules in the stored profile.
**Action:** Reverted the attempt to use `std::mem::take` and kept the `.clone()` calls because the values are retained.
**[Optimized String Allocations in IO Loops]**
**Learning:** Found loops doing `let mut line = String::new();` inside before a `read_line`, allocating continuously instead of reusing capacity.
**Action:** Hoisted the string allocation outside the loops and called `.clear()` to reuse the memory, ensuring zero allocations in the hot read loops.

## 2024-04-27 - [Avoid String clones in RTA DraftCandidate]
**Learning:** Avoid unnecessary `.clone()`s of `String` during serialization workflows.
**Action:** Used `&'a str` in structs purely meant for serialization like `DraftCandidate` and `DraftReport` instead of cloning `String` on every candidate instantiation in hot-ish parsing paths. Also optimized `push_split` by constructing the `SplitEvent` structure and then pushing it to the vector without cloning the original string name argument.

**Replacing push_str format allocations**
**Learning:** Using `string.push_str(&format!(...))` inside loops causes a new String allocation on the heap for every iteration.
**Action:** Use `writeln!(string, ...)` via `std::fmt::Write` to append formatted text directly to the existing buffer without intermediate heap allocations.

**[Pre-allocated CalibrationRecorder Frames]**
**Learning:** `CalibrationRecorder::new` was initializing `frames` with `VecDeque::new()` without reserving capacity, causing repeated heap reallocations as the buffer filled up to 30,000 frames.
**Action:** Used `VecDeque::with_capacity(30_000)` to eliminate reallocations during RTA draft recording.
**[Lazy Iterator Operations]
**Learning:** `Iterator::count()` iterates over all elements to calculate the length. If the result is only needed in an optional branch (like a callback function), it introduces unnecessary O(N) overhead.
**Action:** Wrap eager iterator operations like `count()` inside `if option.is_some() { Some(...) } else { None }` so it's only lazily computed when strictly necessary.

**[Unnecessary Vec Allocations in Tests]
**Learning:** Found several test cases (`should_compute_apu_write_trace_hash`) creating multiple temporary heap allocations (`writes.clone()`) just to mutate a single field for negative assertions.
**Action:** Replace `Vec::clone()` with in-place mutable updates using a `let mut writes = writes;` and reverting the state after assertion, effectively removing 7 heap allocations per test run.

## 2025-02-18 - Eliminated MMC5 CHR Region Heap Allocations
**Learning:** Returning small variable-sized collections via `Vec` introduces significant overhead on hot paths, especially state restoration or mapper syncing, by hitting the allocator.
**Action:** Replace small `Vec` return types (up to N elements) with `([T; N], usize)` tuples. It drastically reduces heap pressure while cleanly integrating with iteration using slice bounds `&regions[..count]`.
