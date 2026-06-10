# 🔭 Vantage: Spec for Feature Flag DX

## 👤 User Story
"As a Developer, I want clear feedback when trying to run experimental features (like `story_demo`), so that I don't waste time diagnosing compiler errors caused by missing feature flags."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, users exploring experimental code face compilation failures such as "NarrativeGenerator not found" if they don't explicitly enable the `nova` feature, as highlighted by our UX testing (`ECHO_NOVA_REPORT.md`). This creates immediate onboarding friction and discourages exploration of new, experimental capabilities. By improving Feature Flag Developer Experience (DX), we make it easy and obvious for users to run these experiments, which ultimately drives engagement and feedback for new capabilities before they are promoted to the core build.

## 📊 Success Metrics
- **Onboarding Success:** 100% of developers attempting to run experimental examples without correct feature flags receive a helpful, actionable error message rather than a raw "symbol not found" compilation error.
- **Reduced Friction:** No confused issues opened about missing types for experimental targets.

## 🕵️ Gap Analysis
- **Market View:** High-quality Rust repositories typically use `compile_error!` or conditionally compiled stubs that prompt the user to enable the correct Cargo feature when running specific examples or bins.
- **Our Gap:** We rely on the user to know that certain bins or modules are implicitly hidden behind the `nova` feature, leading to confusing `rustc` errors if they try to build or run them directly without adding `--features nova`.

## ✅ Acceptance Criteria
- Must provide a clear warning or error mechanism that informs the user to add `--features nova` when attempting to build or run an experimental target without the required feature flag.
- Must apply this mechanism across all existing and future experimental binaries and examples.
- Must update the main README to explicitly mention that experimental features require the `nova` flag.

## 🚫 Out of Scope
- Restructuring the entire feature flag architecture.
- Automatically enabling feature flags (which Cargo does not support dynamically).
