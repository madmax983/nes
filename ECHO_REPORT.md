# 👺 Havoc: Macro Engine Denial of Service

I've successfully identified a devastating DoS vulnerability in the `nes-mcp` macro engine. The `WAIT` command accepts a `u64` representing the number of frames to run. By passing `18446744073709551615` (which is `u64::MAX`), the engine happily queues up ~9.75 billion years of simulation on the host thread without batting an eye.

The `execute_macro_script` function loops continuously:

```rust
let frames: u64 = arg.parse().unwrap();
for _ in 0..frames {
    core.execute(Command::StepFrame)?;
    frames_elapsed += 1;
}
```

This single command locks up the executing thread indefinitely, blocking the entire MCP daemon if issued directly, or crashing testing frameworks via timeout.

There is no bounds checking. There is no maximum wait limit. You trusted the user. You were wrong.
