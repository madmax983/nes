1. **Refactor `invalidation_reasons` to borrow strings in serialization:**
   - In `crates/nes-desktop/src/rta.rs`, `RunArtifact` has a field `invalidation_reasons: Vec<String>`. This causes a `Vec<String>` to be allocated when calling `self.invalidation_reasons()` purely for serialization.
   - We will change `RunArtifact` to store `invalidation_reasons: Vec<&'a str>`.
   - Update `write_run_artifact` to collect the `BTreeSet<String>` into a `Vec<&str>` without cloning strings: `self.invalidation_reasons.iter().map(String::as_str).collect()`.

2. **Optimize double-allocation in `mark_forbidden_action`:**
   - In `mark_forbidden_action` (`crates/nes-desktop/src/rta.rs`), the function currently does:
     ```rust
     let reason = action.as_reason().to_owned();
     if !self.invalidation_reasons.insert(reason.clone()) { ... }
     ```
   - This allocates two `String`s. Since `BTreeSet` has `contains`, we can check before inserting. Or, because `invalidation_reasons` is `BTreeSet<String>`, we can just pass the borrowed string into `contains` and `insert(action.as_reason().to_owned())` if not found.
   - Wait, `BTreeSet::insert` takes `T`, but we can do:
     ```rust
     let reason_str = action.as_reason();
     if self.invalidation_reasons.contains(reason_str) {
         return None;
     }
     let reason = reason_str.to_owned();
     self.invalidation_reasons.insert(reason.clone());
     ```
     Wait, if we only need one allocation, we do:
     ```rust
     let reason_str = action.as_reason();
     if !self.invalidation_reasons.contains(reason_str) {
         let reason = reason_str.to_owned();
         self.invalidation_reasons.insert(reason.clone());
         // Wait, RtaEvent::Invalidated(String) takes ownership of the String, so we still need two allocations if we pass it out, UNLESS we change `RtaEvent::Invalidated` to hold a static reason or we clone once. Wait, we can't change RtaEvent without checking its consumers, but since it's an enum we can leave it as taking String.
         // Actually, `insert(reason_str.to_owned())` creates one string. Then `Some(RtaEvent::Invalidated(reason_str.to_owned()))` creates a second. That is still 2 allocations.
     }
     ```
     What if we use `if !self.invalidation_reasons.insert(reason_str.to_owned())`? That allocates once to pass to `insert`. If it returns false, it drops the allocation. If it returns true, we allocate a *second* time for `RtaEvent::Invalidated`. So it's 1 or 2 allocations depending on if it's new. Wait, if it's already there, `action.as_reason().to_owned()` is a wasted allocation! We can use `contains` first:
     ```rust
     let reason_str = action.as_reason();
     if self.invalidation_reasons.contains(reason_str) {
         return None;
     }
     self.invalidation_reasons.insert(reason_str.to_owned());
     let event_reason = reason_str.to_owned();
     ```
     This prevents allocations on the *hot path* when the reason is already present (e.g., repeatedly doing a forbidden action).

3. **Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.**
4. **Submit PR "⚡ Bolt: Eliminate heap allocations in RTA engine hot paths"**
