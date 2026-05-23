## 2024-05-23 - Handle Invalid Tool Argument Types in MCP Daemon
**Mutant:** Missing type checking when extracting tool arguments
**Diagnosis:** Tests were not fully covering invalid `arguments` extraction where `arguments` was a scalar or array rather than an object.
**Kill Shot:** Added tests to catch when valid parameters pass, and invalid parameter structures are properly denied instead of swallowed.

## 2024-05-23 - Payload Size Boundary Tests in `nes-desktop` MCP Host
**Mutant:** Weak boundary checks around `MAX_PAYLOAD_SIZE`
**Diagnosis:** Boundary conditions like length perfectly matching limits or just exceeding limits were not actively covered by failing tests, allowing mutant changes to `>` vs `>=` to survive.
**Kill Shot:** Expanded test logic to cover the exact boundary limits, testing both well within limit and off-by-one.

## 2024-05-23 - JSON RPC error handling tests in `nes-desktop` MCP Host
**Mutant:** Uncaught paths when deserializing bad JSON or bad RPC version
**Diagnosis:** Mutations causing incorrect responses when clients submit fully malformed JSON, missing ID paths, or incompatible JSON-RPC versions survived because testing relied predominantly on well-formed requests.
**Kill Shot:** Added unit tests directly validating responses when submitting invalid JSON and unsupported jsonrpc versions.

**[EOF Read Loop Timeout]**
**Mutant:** replace == with != in `read_framed_message` (`read == 0`) in crates/nes-desktop/src/mcp_host.rs
**Diagnosis:** Equivalent Mutant. Altering the EOF read check (`read == 0`) into continuous loops results in TIMEOUT. This is an expected weakness based on how test runners enforce time limits.
**Kill Shot:** None. This is documented as an expected limitation.
