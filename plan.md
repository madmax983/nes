We can refactor `push_split` in `crates/nes-desktop/src/rta.rs` to optimize how the `SplitEvent` is constructed and passed to avoid redundant code and memory usage.

```rust
<<<<<<< SEARCH
    fn push_split(
        &mut self,
        name: String,
        source: SplitSource,
        frame: u64,
        now: Instant,
    ) -> SplitEvent {
        self.split_counter = self.split_counter.saturating_add(1);
        let elapsed_ms = self.elapsed(now).as_millis();
        self.split_events.push(SplitEvent {
            name: name.clone(),
            source,
            frame,
            elapsed_ms,
        });
        SplitEvent {
            name,
            source,
            frame,
            elapsed_ms,
        }
    }
=======
    /// ⚡ Bolt Optimization:
    /// Constructs the `SplitEvent` struct once and pushes a `.clone()` of it into the collection,
    /// rather than instantiating two identical structs and independently cloning individual fields like `String`.
    fn push_split(
        &mut self,
        name: String,
        source: SplitSource,
        frame: u64,
        now: Instant,
    ) -> SplitEvent {
        self.split_counter = self.split_counter.saturating_add(1);
        let elapsed_ms = self.elapsed(now).as_millis();

        let event = SplitEvent {
            name,
            source,
            frame,
            elapsed_ms,
        };

        self.split_events.push(event.clone());
        event
    }
>>>>>>> REPLACE
```
