1. Refactor `api.rs` to flatten nested `match` statements in `LoadedMapper`.
   - Replaced redundant `match` blocks for each method with `get_mapper` and `get_mapper_mut` helper functions that return `&dyn Mapper` and `&mut dyn Mapper`.
2. Ensure no logic has changed by running the test suite (`cargo test`).
3. Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
