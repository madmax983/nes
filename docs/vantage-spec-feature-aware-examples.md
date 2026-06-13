# 🔭 Vantage: Spec for Feature-Aware Examples

## 👤 User Story
"As a Developer evaluating the library, I want example binaries and doctests to clearly state when a feature flag is required, so that I don't waste time debugging `unresolved import` or `module not found` errors when copying code from the README or docs."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, when a user tries to run a documented example that relies on an optional or experimental feature (like `nova`), the compiler simply fails with confusing missing type/module errors. This creates immediate onboarding friction, as highlighted by our UX testing (`ECHO_NOVA_REPORT.md`), causing users to abandon the library out of frustration. By making examples "feature-aware" and ensuring clear error messaging when features are missing, we drastically reduce time-to-first-success, improving developer experience and library adoption.

## 📊 Success Metrics
- **Onboarding Success:** 100% of standard examples either compile successfully out-of-the-box or provide a human-readable compiler error explicitly naming the missing required feature flag.
- **Reduced Friction:** Zero reported issues of "type not found" when users attempt to run official examples.

## 🕵️ Gap Analysis
- **Market View:** High-quality Rust crates use `#[cfg(feature = "...")]` combined with `compile_error!` or `std::compile_error!` in examples, or use `required-features` in `Cargo.toml` to gracefully skip or explain disabled examples.
- **Our Gap:** We document experimental features but do not enforce guardrails in the example binaries themselves, leading to raw compiler failures when users omit `--features nova`.

## ✅ Acceptance Criteria
- Must update all example binaries (e.g., `story_demo`) and doctests that rely on non-default features (like `nova`) to gracefully handle missing features.
- If a required feature is disabled, the code must trigger a clear, custom compiler error (e.g., `compile_error!("This example requires the 'nova' feature. Run with --features nova")`) rather than failing with generic unresolved imports.
- Alternatively, examples can use `required-features = ["nova"]` in their `Cargo.toml` `[[example]]` definitions so `cargo run --example` provides Cargo's native missing feature warning.
- The `README.md` must clearly tag examples that require non-default features.

## 🚫 Out of Scope
- Enabling all experimental features by default.
- Rewriting the experimental features themselves (this is strictly about the developer experience of the examples).
