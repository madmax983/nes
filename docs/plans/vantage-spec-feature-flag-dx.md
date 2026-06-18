# 🔭 Vantage: Spec for Feature Flag Developer Experience (DX)

## 👤 User Story
"As a Developer or advanced User exploring experimental binaries (like `story_demo`), I want clear, actionable instructions or compiler errors when a required feature flag is missing, so that I don't waste time diagnosing cryptic 'not found' errors."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, when a user tries to compile an experimental tool gated behind a feature flag (like `nova`), the Rust compiler emits confusing `unresolved import` or `type not found` errors (e.g., `NarrativeGenerator` not found) instead of a clear explanation. This creates developer friction, decreases adoption of our experimental tooling, and leads to wasted time and unnecessary support requests, as reported in `docs/ECHO_NOVA_REPORT.md`. A seamless DX for optional features ensures users can self-serve and successfully test our new capabilities.

## 📊 Success Metrics
- **Zero Support Queries:** Eliminate support questions related to "not found" compiler errors for feature-gated code.
- **Fast Recovery:** Users encountering a missing feature flag resolve the issue and successfully compile the code in under 1 minute.

## 🕵️ Gap Analysis
- **Market View:** World-class Rust libraries (like `tokio` or `reqwest`) use explicit `compile_error!` macros or detailed `#[cfg(...)]` attributes to inform users exactly which feature flag needs to be enabled to access a specific type or binary.
- **Our Gap:** We silently omit the types when the `nova` feature is not enabled, leading to generic Rust compiler resolution errors that do not mention the `nova` feature at all. Furthermore, the `README.md` lacks explicit feature flags in the demonstration commands.

## ✅ Acceptance Criteria
- Code that is heavily reliant on a specific feature flag (e.g., `story_demo` requiring `nova`) must include a `compile_error!("The 'nova' feature must be enabled to build this target.");` if the feature is not active.
- The root `README.md` must be updated to include a prominent warning/banner regarding experimental features and their required flags (e.g., `REQUIRES FEATURE NOVA`).
- Example launch commands in the documentation for experimental features must explicitly include the `--features` argument (e.g., `cargo run --features nova`).

## 🚫 Out of Scope
- Removing the `nova` feature flag entirely and merging experimental features into the stable release.
- Implementing automatic dependency resolution or dynamic feature activation at runtime.
