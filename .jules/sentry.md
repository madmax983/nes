## 2025-05-15 - ROM Parsing Missing Edge Cases
**Learning:** Even though the happy path (valid ROMs) was tested, several invalid format variations of ROM files were missing coverage.
**Action:** Always write tests targeting the error variants. When the crate returns an explicit Enum like `RomError`, ensure every variant is triggered at least once.

## 2025-05-15 - Testing TAS Record/Run Coalescing
**Learning:** Run aggregation bugs inside recording modules (like `TasMovie`) can easily manifest if edge cases like adding 0-frames or overflowing the frame counters aren't checked explicitly.
**Action:** When working with types encapsulating arrays of structured runs, push tests targeting the zero-op insertion, bounds checks, and overflow boundaries.

## 2024-05-24 - [MemoryHeatmap Test Coverage]
**Finding:** Uncovered code in `MemoryHeatmap::new` was resolved by adding initialization tests using default parameters and verifying correct sizing of the heap allocations.
**Action:** Always verify `new()` implementations on experimental visualizers and ensure initialization defaults behave exactly as documented. Ensure coverage is maintained on utility/experimental tools as regressions there often signal core breakage later on.

## 2024-05-27 - [MCP Content-Length Capacity Overflow]
**Learning:** `std::vec::Vec::with_capacity` or `vec![0_u8; len]` calls where `len` is derived from an untrusted client stream header (like `Content-Length`) can easily trigger process-terminating Out-Of-Memory/capacity overflow panics if not bounded.
**Action:** When inspecting IO/stream parsers, actively look for allocations that map 1:1 with unvalidated incoming lengths. Establish `MAX_PAYLOAD_SIZE` ceilings and test with excessively large payloads to ensure gracefully handled `Err` results rather than `panic`.

## 2024-05-27 - [MCP Slowloris DoS Vulnerability]
**Learning:** A synchronous TCP listener loop that performs blocking reads (or allows infinite idle time) on a client socket is vulnerable to a Slowloris attack, where a single malicious client can tie up the entire server. Simply adding a blanket `read_timeout` to the socket breaks persistent connections (like JSON-RPC/LSP).
**Action:** Always process client connections concurrently (e.g., using `thread::spawn` or an async runtime) to prevent head-of-line blocking on the listener thread, while preserving the ability for legitimate connections to remain idle safely.

## 2025-05-27 - [Unbounded Stream Input Vulnerability]
**Learning:** Functions that parse IO Streams with `std::fs::read` (e.g. `load_state_file`, `load_rom_session`) are vulnerable to Out-Of-Memory (OOM) SIGKILL crashes if they are handed infinite streams like `/dev/zero`. Unbounded `fs::read` will continuously allocate memory until the process is forcefully killed.
**Action:** When working with file readers where the size is untrusted or unbounded, replace `fs::read` with a `bounded_read` utility using `std::io::Read::take(max_size).read_to_end()` to cap allocations. Assert expected errors in tests to verify this limit, instead of using `#[ignore]` on OOM crashes.
