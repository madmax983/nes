1. **Refactor `resolve_runtime_config` in `crates/nes-desktop/src/config.rs`:**
   - **Smell:** `resolve_runtime_config` is ~126 lines long and mixes configuration loading, precedence logic (CLI vs config file), fallback paths, validation, and object construction for several sub-components (like RTA and Netplay).
   - **Solution:** Extract the RTA (`resolve_rta_config`) and Netplay (`resolve_netplay_config`) configuration logic into private helper functions. This will shorten `resolve_runtime_config`, making the precedence rules for the main application config clearer while isolating the sub-module logic.
   - **Benefit:** Dramatically improves readability, reduces the function length to a manageable size, and scopes variables tighter. No behavior change.
2. **Review & Test:**
   - Ensure the new helper functions are properly called and return the `Option<ConfigType>` precisely as they currently evaluate.
   - Add journal entry into `.jules/forge.md` recording the specific extraction of nested configurations into helpers.
   - Run `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` to guarantee the refactor changes no logic.
3. **Pre-commit:**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
4. **Submit PR:**
   - PR Title: `⚒️ Forge: Extract config resolution helpers`
   - Description matching Forge constraints exactly.
