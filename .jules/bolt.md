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

**[VecDeque for FIFO Collections]**
**Learning:** When using a `Vec` as a FIFO queue (e.g., removing the oldest element with `.remove(0)` on every frame), it triggers an O(n) memory shift of all subsequent elements. This scales poorly for large capacities like 30,000 frames.
**Action:** Use `VecDeque` with `push_back` and `pop_front` instead of `Vec` for fixed-capacity circular buffers to reduce O(n) shifts to O(1) operations.
