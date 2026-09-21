# 🗣️ Echo: RelayArgs example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the `RelayArgs` example from the `nes-relay` documentation. The compiler threw an error: `missing field metrics_addr in initializer of RelayArgs`.

* 🕵️ **The Reality:** Turns out the `RelayArgs` struct had a new field `metrics_addr` added (likely recently), but the documentation example wasn't updated to include it.

* 💡 **The Fix:** Update the doc example in `crates/nes-relay/src/config.rs` to include `metrics_addr: None`.
