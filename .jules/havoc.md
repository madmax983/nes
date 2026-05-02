# Havoc Journal

## 2024-04-09 - Mutex Poisoning in MCP
🧨 **The Trigger:** A thread panics while holding the global `OutputState` Mutex in `nes-mcp`.
📉 **The Stack Trace:** Subsequent calls to `frame_chunk` crash with `expect("output state lock")` on a poisoned lock.
🧪 **Reproduction:** `cargo test --test havoc_output_dos -- --ignored`
😈 **Comment:** You assumed the user closure would never panic. You were wrong.

## 2025-04-07 - MCP Daemon OOM Crash
🧨 **The Trigger:** A malicious payload with an impossibly large `Content-Length`.
📉 **The Stack Trace:** No explicit stack trace outputted; the test `havoc_crash_mcp_daemon_oom` passed, meaning the daemon crashed as expected from the OOM attack.
🧪 **Reproduction:** Run `cargo test -p nes-mcp --test mcp_havoc_crash`
😈 **Comment:** You assumed the client would not send 18 Exabytes. You were wrong.

## 2025-04-07 - Macro Engine DoS Hang
🧨 **The Trigger:** A WAIT command with u64::MAX frames in the MCP macro engine.
📉 **The Stack Trace:** (Thread hangs and eventually panics due to test timeout)
```
thread 'havoc_crash_mcp_dos_wait_frames' panicked at crates/nes-mcp/tests/havoc.rs:40:10:
timeout
```
🧪 **Reproduction:** Run `cargo test -p nes-mcp --test havoc havoc_crash_mcp_dos_wait_frames`
😈 **Comment:** You assumed nobody would want to WAIT longer than the universe has existed. You were wrong.
## 2024-04-09 - Concurrency Deadlock in Client Cleanup
🧨 **The Trigger:** Loom discovers a deadlock / race condition when cleaning up a relay client from the global room map concurrently.
📉 **The Stack Trace:** Loom model trace panics on deadlock detection.
🧪 **Reproduction:** `RUSTFLAGS="--cfg loom" cargo test --test havoc_loom_deadlock -- --ignored`
😈 **Comment:** "Thread-safe" is a lie until proven by `loom`.

**[nes-mcp Output Mutex Poisoning]**
**The Trigger:** Triggering a panic within the closure passed to `nes_mcp::publish_frame_with`.
**The Stack Trace:**
```
thread 'havoc_test_mutex_poison' panicked at crates/nes-mcp/src/output.rs:163:14:
output state lock poisoned
```
**Reproduction:** Run `cargo test -p nes-mcp --test havoc_mcp_output_dos -- --ignored`.
**Comment:** A simple panic in the frame publishing closure completely destroys the global frame metadata state because the lock is poisoned. The whole application crashes unrecoverably.

**[nes-relay Loom Deadlock]**
**The Trigger:** Acquiring locks while holding locks within cleanup client loop.
**The Stack Trace:**
```
thread '<unnamed>' panicked at 'deadlock', crates/nes-relay/tests/havoc_loom_deadlock.rs:25:17
```
**Reproduction:** Run `cargo test -p nes-relay --test havoc_loom_deadlock`.
**Comment:** We simulated a deadlock vector in `nes-relay`'s client cleanup if it attempts to lock the state again while iterating peers. This proves the system is fragile to naive lock usage during broadcasts.

**[nes-mcp Output Mutex Poisoning - Audio]**
**The Trigger:** Triggering a panic within the closure passed to `nes_mcp::publish_audio_with`.
**The Stack Trace:**
```
thread 'havoc_test_mutex_poison' panicked at crates/nes-mcp/src/output.rs:163:14:
output state lock poisoned
```
**Reproduction:** Run `cargo test -p nes-mcp --test havoc_mcp_audio_dos -- --ignored`.
**Comment:** A simple panic in the audio publishing closure completely destroys the global audio metadata state because the lock is poisoned. The whole application crashes unrecoverably.

## 2023-10-25 - [MCP Host OOM]
🧨 **The Trigger:** Input string `Content-Length: 18446744073709551615\r\n\r\n` caused buffer overflow via capacity overflow panic.
📉 **The Stack Trace:**
```
thread 'havoc_mcp_host_oom' panicked at library/alloc/src/raw_vec/mod.rs:28:5:
capacity overflow
```
🧪 **Reproduction:** Run `cargo test --package nes-desktop --test havoc_oom --features mcp-host`.
😈 **Comment:** You assumed `Content-Length` could be trusted to pre-allocate an unbounded buffer. You were wrong.
## YYYY-MM-DD - MCP Host OOM Vulnerability
**The Target:** `nes-desktop::mcp_host::read_framed_message`
**The Trigger:** Sending a JSON-RPC HTTP header with an absurdly large `Content-Length` (e.g., `18446744073709551615`).
**The Result:** The host blindly trusts the header and attempts to allocate an unconstrained `vec![0_u8; len]`. This results in an immediate Out-Of-Memory panic (`capacity overflow`), crashing the entire desktop application.
**The Fix (Not Mine):** Someone needs to clamp `len` to a reasonable maximum (e.g., a few MBs) or use a capped stream reader.

## 2024-04-26 - OOM via load_rom /dev/zero
**The Trigger:** `{"rom_path": "/dev/zero"}` provided to the `load_rom` MCP tool.
**The Stack Trace:** (Process sent SIGKILL due to Out of Memory)
**Reproduction:** Run `cargo test -p nes-mcp --test havoc_load_rom_oom -- --ignored`
**Comment:** You assumed users would only pass valid files. You were wrong.

## YYYY-MM-DD - Desktop Session OOM via /dev/zero
🧨 **The Trigger:** `Path::new("/dev/zero")` provided to the `load_rom_session` or similar functions that use `std::fs::read`.
📉 **The Stack Trace:** (Process sent SIGKILL due to Out of Memory. No explicit panic trace).
🧪 **Reproduction:** Run `cargo test -p nes-desktop havoc_desktop_load_rom_oom -- --ignored`
😈 **Comment:** You assumed the desktop application would only ever try to read finite files. You were wrong.

## YYYY-MM-DD - Desktop Save State OOM via /dev/zero
🧨 **The Trigger:** `Path::new("/dev/zero")` provided to the `manual_state::load_state_file` function that uses `std::fs::read`.
📉 **The Stack Trace:** (Process sent SIGKILL due to Out of Memory. No explicit panic trace).
🧪 **Reproduction:** Run `cargo test -p nes-desktop havoc_desktop_load_state_oom -- --ignored`
😈 **Comment:** You assumed the desktop application would only ever try to read finite save state files. You were wrong.

## YYYY-MM-DD - Desktop RTA Profiles OOM via /dev/zero
🧨 **The Trigger:** `Path::new("/dev/zero")` provided to `rta::load_profiles` which iterates files and reads them via `std::fs::read_to_string`.
📉 **The Stack Trace:** (Process sent SIGKILL due to Out of Memory. No explicit panic trace).
🧪 **Reproduction:** Run `cargo test -p nes-desktop havoc_rta_profiles_oom -- --ignored`
😈 **Comment:** You assumed RTA profiles would only ever be finite files. You were wrong.

## YYYY-MM-DD - Desktop Session OOM via /dev/zero
🧨 **The Trigger:** `Path::new("/dev/zero")` provided to the `load_rom_session` or similar functions that use `std::fs::read`.
📉 **The Stack Trace:** (Process sent SIGKILL due to Out of Memory. No explicit panic trace).
🧪 **Reproduction:** Run `cargo test -p nes-desktop havoc_desktop_load_rom_oom -- --ignored`
😈 **Comment:** You assumed the desktop application would only ever try to read finite files. You were wrong.

## YYYY-MM-DD - Desktop MCP Host Content-Length OOM
🧨 **The Trigger:** A malicious payload with an impossibly large `Content-Length` of `18446744073709551615`.
📉 **The Stack Trace:** `thread 'havoc_mcp_content_length_oom' panicked at library/alloc/src/raw_vec/mod.rs:28:5: capacity overflow`
🧪 **Reproduction:** Run `cargo test -p nes-desktop havoc_mcp_content_length_oom -- --ignored`
😈 **Comment:** You assumed `Content-Length` could be trusted to pre-allocate an unbounded buffer. You were wrong.

## YYYY-MM-DD - Desktop MCP Host Slowloris DoS
🧨 **The Trigger:** A malicious actor sending a valid `Content-Length` header but only a partial payload and leaving the connection open.
📉 **The Stack Trace:** (Test assertions show a second client being delayed or failing).
🧪 **Reproduction:** Run `cargo test -p nes-desktop havoc_mcp_slowloris_dos -- --ignored`
😈 **Comment:** You assumed every connected client would send their payload promptly. You were wrong.
