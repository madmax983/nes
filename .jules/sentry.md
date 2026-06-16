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

## 2026-05-05 - Edge Cases in Image Encoding and Script Generation
**Learning:** Error returns on extremely large dimensions (overflow) in utilities like PPM encoding and unimplemented player 2 script generation features are easily missed in standard coverage.
**Action:** Write tests specifically targeting dimension overflows and verify error variants for unsupported inputs are triggered.

## 2026-05-09 - Testing precise bitwise values for hashes and cheat codes
**Learning:** Checking for mere non-zero output `assert_ne!(hash, 0)` is insufficient for bitwise operations (`|`, `&`, `^`) in hashes or parsers, as multiple operators can produce non-zero or identical outputs. For instance, `|` and `^` are functionally identical if the operands do not have overlapping bits set.
**Action:** When testing bitwise hash/parsing logic, calculate and `assert_eq!` the exact expected bit pattern instead of just non-zero outputs to effectively kill mutants.
## 2025-06-18 - Missing cfg(feature = "mcp-host") and PPM edge cases
**Learning:** The tests checking mcp-host features in `nes-desktop` (e.g. `mcp_host_slowloris`, `havoc_mcp_oom`) were not gated behind `#[cfg(feature = "mcp-host")]`, which caused `unresolved import` compilation errors when tested locally or without `--all-features`. Furthermore, `encode_ppm` was lacking test coverage for length mismatch due to dimension configurations (e.g. height zero).
**Action:** Always add the proper feature gate on the top level of feature-dependent test files. Additionally, filled the `encode_ppm` gap with a new test `encode_ppm_returns_error_on_height_zero`.
