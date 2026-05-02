# Forge's Journal

**[Main Run Refactor]**
**Learning:** `run` functions or main event loops tend to accumulate massive setup logic making them difficult to follow.
**Action:** Extract configuration parsing, component initialization, and event handling into distinct, focused helper functions.

**[Extract Nested Configuration Setup]**
**Learning:** Large application setup functions (`resolve_runtime_config` or `main`) often accumulate configuration parsing, precedent rules, and struct construction for isolated sub-components (like RTA or Netplay), making the core path difficult to read and tightly coupling scopes.
**Action:** Extract the isolated configuration logic into pure, private helper functions (e.g., `resolve_netplay_config`, `resolve_rta_config`). This scopes variables tightly and visually separates the "what to initialize" from the "how to evaluate the inputs".
