## 2025-03-26 - [BTreeMap pop_first optimization]
**Learning:** Removing the smallest key/entry from a `BTreeMap` using `keys().next().copied()` followed by a `remove(&key)` is inefficient because it requires two O(log N) tree traversals (one to find the first element, one to traverse down to remove it by key).
**Action:** Always use the `BTreeMap::pop_first()` method to perform this action in a single step, saving a lookup and avoiding the `.copied()` allocation.
## 2025-06-19 - Removed format! heap allocation on hot IO path

**Learning:** When writing formatted strings (like headers) to an IO writer (like `TcpStream` or `Stdout`), using `writer.write_all(format!("...").as_bytes())` causes an unnecessary intermediate `String` allocation on the heap for every write.
**Action:** Always use the `write!` macro (e.g., `write!(writer, "...", ...)`) from `std::io::Write` instead. It writes directly to the stream without allocating an intermediate string buffer.
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

**[Avoiding heap allocation when writing text into Vec<u8>]**
**Learning:** `extend_from_slice(format!("...").as_bytes())` dynamically allocates a temporary `String`, formats text into it, copies the bytes into the `Vec`, and immediately drops the `String`. This generates an unnecessary heap allocation on the hot path (like encoding image frames).
**Action:** Use `std::io::Write::write!` directly on the `&mut Vec<u8>` (e.g., `write!(&mut ppm, "...").unwrap()`) to format and write the bytes sequentially into the pre-allocated vector without temporary strings.

**Removing String allocation in IO write**
**Learning:** `writer.write_all(format!("...").as_bytes())` dynamically allocates a temporary `String`, formats text into it, copies the bytes into the `writer`, and drops the `String`. This generates an unnecessary heap allocation on every MCP JSON-RPC response.
**Action:** Use `write!(writer, "...")` instead of allocating an intermediate `String` via `format!` to avoid the heap allocation.

**[Overlay Rendering Loop Allocations]
**Learning:** Passing slices `&[T]` to UI rendering loops often forces the caller to eagerly collect mapped states (like strings or toggles) into temporary `Vec`s, causing frequent heap allocations.
**Action:** Use `impl Iterator<Item = T> + Clone` directly in the UI render signature to pull and map states dynamically from the underlying source without allocating intermediate collections.

**[Iterator Clone Performance Pitfall]
**Learning:** Refactoring UI render slices (`&[T]`) to lazy iterators (`impl Iterator<Item = T> + Clone`) successfully eliminates intermediate collections, but if the map closure performs allocations (like `to_string()`), cloning the iterator and traversing it repeatedly (e.g. `.find()`) will trigger $O(N^2)$ allocations, destroying performance.
**Action:** When using iterators to defer collections on the hot path, ensure all expensive operations or heap allocations inside the map closure are fully removed and deferred until the absolute last moment during the final rendering.
**[VecDeque for FIFO Collections]**
**Learning:** When using a `Vec` as a FIFO queue (e.g., removing the oldest element with `.remove(0)` on every frame), it triggers an O(n) memory shift of all subsequent elements. This scales poorly for large capacities like 30,000 frames.
**Action:** Use `VecDeque` with `push_back` and `pop_front` instead of `Vec` for fixed-capacity circular buffers to reduce O(n) shifts to O(1) operations.
**Removing String allocation when string parsing and mapping**
**Learning:** Chaining `.collect::<String>()` on an iterator with operations like `.chars().map(...).collect::<String>()` allocates memory but cannot guarantee a single allocation if the final length isn't perfectly predictable to the allocator, resulting in potential re-allocations on the hot path.
**Action:** Use `String::with_capacity()` to pre-allocate the exact known maximum size, and a `for` loop with `.push()` to append characters manually. This guarantees exactly one heap allocation.

## 2025-06-19 - Replace O(N) allocation in DSL parser with peekable iterator
**Learning:** Found an unnecessary intermediate heap allocation `let chars: Vec<char> = line.chars().collect();` in `nes-dsl/src/lib.rs` while doing string look-ahead for comment stripping.
**Action:** Replaced the collected vector with `let mut chars = line.chars().peekable();`. A peekable iterator allows the look-ahead behavior needed (`chars.peek() == Some(&'/')`) without needing to load the entire line into memory.

## 2025-06-19 - Removed format! and Vec::new() on hot emulator path
**Learning:** Returning `String` and `Vec<u8>` from inner parsing routines on the CPU emulation hot loop causes multiple allocations per executed opcode, degrading performance.
**Action:** Replace `format!` and `vec![]` with `format_args!` and array slices (`&[u8]`). Pass `fmt::Arguments` to diagnostic functions that only evaluate and allocate them when tracing is explicitly enabled.

## 2026-04-08 - [Iterator Clone Performance Pitfall in Overlay]
**Learning:** The UI rendering loop repeatedly called `.clone()` on an iterator mapping states. Even if mapping is cheap, cloning and traversing $O(N^2)$ times inside the frame renderer generates unnecessary overhead.
**Action:** Pre-collect small UI list states (like slot summaries) into fixed-size stack arrays `[Option<T>; 5]` outside the rendering loop, eliminating iterator clones and reducing lookups to $O(1)$ without heap allocations.
