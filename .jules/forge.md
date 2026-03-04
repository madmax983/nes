# Forge's Journal

**[nes-dsl Finalize Bug Fixed]
**Learning:** Found a panic-on-overwrite bug during forward reference resolution in `nes-dsl` caused by `self.write_resolved_byte(...)`.
**Action:** Always favor UPSERT (`insert(...)`) style resolution instead of strictly asserting `write_resolved_byte` matches over initial placeholder values when assembling.
