# 🔭 Vantage: Spec for Nova Feature Discoverability

## 👤 User Story
"As a Developer exploring the R&D features, I want clear, in-code documentation and compiler hints when trying to use experimental modules, so that I don't waste time diagnosing 'module not found' errors."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, when developers attempt to use experimental `nova` features (like `story_demo` or `NarrativeGenerator`), they encounter abrupt compiler errors (`module not found`) if the `nova` feature flag isn't explicitly passed. This causes significant confusion and degrades the developer experience (`ECHO_NOVA_REPORT.md`). The proposed "Fix" of adding a huge banner to the README is brittle and easily missed. By solving this systemically at the code level, we improve discoverability, reduce onboarding friction for R&D contributors, and keep our experimental surface area self-documenting.

## 📊 Success Metrics
- **Discoverability:** 100% of developers attempting to compile `nova` feature examples without the flag enabled receive a clear, actionable error message instructing them to enable the feature.
- **Zero Confusion:** No generic "not found" compiler errors are presented to users trying to access defined but feature-gated experimental APIs.

## 🕵️ Gap Analysis
- **Market View:** High-quality Rust libraries (like `tokio` or `serde`) use `compile_error!` macros or `doc(cfg)` annotations to clearly communicate when a required feature flag is missing.
- **Our Gap:** We silently omit experimental code via `#[cfg(feature = "nova")]`, leaving the compiler to throw unhelpful "not found" errors when users attempt to follow documentation or examples.

## ✅ Acceptance Criteria
- When a user attempts to compile an example or binary that strictly relies on `nova` features (e.g., `story_demo`), the build must fail with a custom, human-readable `compile_error!` (e.g., "The 'nova' feature must be enabled to use this example.") rather than a generic module resolution error.
- Alternatively, public-facing documentation for these features must clearly indicate the required `nova` feature flag (using `#![feature(doc_cfg)]` if applicable, or explicit doc comments).
- The core emulator build (`nes-desktop`, etc.) must remain unaffected and compile cleanly without the `nova` feature.

## 🚫 Out of Scope
- Enabling the `nova` feature by default in the workspace.
- Refactoring the underlying implementation of `NarrativeGenerator` or `story_demo`.
