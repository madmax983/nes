🚮 Smell: The `resolve_runtime_config` function in `crates/nes-desktop/src/config.rs` was ~126 lines long and mixed configuration loading, precedence logic (CLI vs config file), fallback paths, validation, and object construction for several sub-components (like RTA and Netplay).
✨ Solution: Extracted the Netplay (`resolve_netplay_config`) and RTA (`resolve_rta_config`) configuration logic into clean, private helper functions. Also ensured correct referencing of `RuntimeArgs`.
🧼 Benefit: Dramatically improves readability, reduces the function length to a manageable size, and scopes variables tighter. No behavior change.
🛡️ Verification: Tests passed. No logic changed.
