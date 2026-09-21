1. **Add `smallvec` dependency to `nes-desktop/Cargo.toml`**
   - Run `sed -i 's/^crossterm = "0.28"/crossterm = "0.28"\nsmallvec = "1.13"/' ./crates/nes-desktop/Cargo.toml` to add `smallvec` dependency.
2. **Verify `Cargo.toml` modification**
   - Run `grep "smallvec" ./crates/nes-desktop/Cargo.toml` to verify the dependency was added successfully.
3. **Change return type of `RtaManager::tick` to use `smallvec::SmallVec` in `rta.rs`**
   - Create a Python script `patch_rta.py` using a here-doc, execute it, and remove it:
```bash
cat << 'PYEOF' > patch_rta.py
with open('./crates/nes-desktop/src/rta.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub fn tick<F>(&mut self, frame: u64, now: Instant, mut read_u8: F) -> Vec<RtaEvent>',
    'pub fn tick<F>(&mut self, frame: u64, now: Instant, mut read_u8: F) -> smallvec::SmallVec<[RtaEvent; 2]>'
)
content = content.replace(
    'let mut events = Vec::<RtaEvent>::with_capacity(2);',
    'let mut events = smallvec::SmallVec::<[RtaEvent; 2]>::new();'
)
content = content.replace(
    'fn tick_start<F>(&mut self, now: Instant, mut read_u8: F, events: &mut Vec<RtaEvent>)',
    'fn tick_start<F>(&mut self, now: Instant, mut read_u8: F, events: &mut smallvec::SmallVec<[RtaEvent; 2]>)'
)
content = content.replace(
    'fn tick_pause_resume<F>(\n        &mut self,\n        now: Instant,\n        mut read_u8: F,\n        events: &mut Vec<RtaEvent>,\n    ) -> bool',
    'fn tick_pause_resume<F>(\n        &mut self,\n        now: Instant,\n        mut read_u8: F,\n        events: &mut smallvec::SmallVec<[RtaEvent; 2]>,\n    ) -> bool'
)
content = content.replace(
    'fn tick_splits<F>(\n        &mut self,\n        frame: u64,\n        now: Instant,\n        mut read_u8: F,\n        events: &mut Vec<RtaEvent>,\n    )',
    'fn tick_splits<F>(\n        &mut self,\n        frame: u64,\n        now: Instant,\n        mut read_u8: F,\n        events: &mut smallvec::SmallVec<[RtaEvent; 2]>,\n    )'
)
content = content.replace(
    'fn tick_end<F>(&mut self, frame: u64, now: Instant, mut read_u8: F, events: &mut Vec<RtaEvent>)',
    'fn tick_end<F>(&mut self, frame: u64, now: Instant, mut read_u8: F, events: &mut smallvec::SmallVec<[RtaEvent; 2]>)'
)

with open('./crates/nes-desktop/src/rta.rs', 'w') as f:
    f.write(content)
PYEOF
python3 patch_rta.py
rm patch_rta.py
```
4. **Verify `rta.rs` modifications**
   - Run `grep -C 3 "SmallVec" ./crates/nes-desktop/src/rta.rs` to verify the modifications.
5. **Run all relevant tests**
   - Run `cargo test -p nes-desktop` to ensure no regressions were introduced.
6. **Complete pre commit steps**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
7. **Submit the change**
   - Call `submit` with the exact PR description:
     ```
     💡 What: Switched `RtaManager::tick` return type from `Vec<RtaEvent>` to `smallvec::SmallVec<[RtaEvent; 2]>`.
     🎯 Why: `RtaManager::tick` is called every frame (hot path) and allocated a `Vec` with capacity 2. RTA events per frame rarely exceed 2, so heap allocations were unnecessary.
     📊 Impact: Eliminates 1 heap allocation per frame (60 allocations per second).
     🔬 Measurement: Run `cargo bench` or profile `RtaManager::tick` to observe zero allocations.
     ```
