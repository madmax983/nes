# 👺 Havoc: Concurrent cleanup_client deadlock in RelayState

* 🧨 **The Trigger:** Calling `cleanup_client` concurrently from multiple threads when a room is not removed. The second thread fails to acquire the `try_lock()` because the first thread holds it, triggering an explicit `panic_any("deadlock")` inside `cleanup_client`.

* 📉 **The Stack Trace:**
```
thread 'havoc_test_loom_cleanup_client_deadlock' (8893) panicked at crates/nes-relay/tests/havoc_loom_deadlock.rs:30:17:
deadlock
```

* 🧪 **Reproduction:** Run `cargo test --test havoc_loom_deadlock`.

* 😈 **Comment:** You assumed that because a lock isn't poisoned it's automatically safe to re-acquire. `try_lock` failed and your code deliberately panics. Next time use a ReentrantMutex or don't try to lock it while you're already holding the lock!
