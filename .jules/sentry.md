## 2025-06-25 - Duplicate Macro Attributes when Injecting Tests
**Learning:** Naive string replacement in Python scripts to inject test functions often duplicates the `#[test]` macro if it matches improperly, causing clippy failures on duplicate attributes.
**Action:** Use AST-aware insertion or strict regex string replacements to sanitize overlapping `#[test]` macros when appending to test modules.

