🗑️ Smell: Boilerplate repetition in core API operations (`execute`, `delta_to`, `apply_delta`, `mapper_hash_component`) and app UI command processing (`execute_app_action`). The lack of macros led to highly duplicated match statements that cluttered files and increased risk of typo-induced errors.

✨ Solution: Introduced targeted local `macro_rules!` to extract the boilerplate. `mix!` simplifies bitwise hashing, `compare_mapper!` and `restore!` handle mapper state iteration safely, and `invalidate_rta!` and `block_if_rta_active!` remove boilerplate bounds checking in the UI layer.

🧩 Benefit: Significantly reduces vertical scroll fatigue, deduplicates identical execution paths, and prevents future developers from accidentally omitting essential boilerplate (like `self.sync_ppu_register_image()`) when adding new logic arms.

🛡️ Verification: Tests passed. No logic changed.
