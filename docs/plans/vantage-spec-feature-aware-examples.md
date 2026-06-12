# 🔭 Vantage: Spec for Feature-Aware Examples

## 👤 User Story
"As a Developer evaluating the codebase, I want examples that depend on specific Cargo features to provide a helpful runtime error or fallback when run without those features, so that I immediately understand how to fix my build rather than hitting cryptic compiler errors about missing structs."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, when users attempt to run experimental examples (like `story_demo`) without explicitly enabling the required feature flags (like `nova`), they are met with catastrophic compiler errors (e.g., "NarrativeGenerator not found"). This creates immense friction during the first 5 minutes of onboarding, causing developers to assume the codebase is broken and abandon the project. By making our examples "feature-aware," we guide users to the correct usage gracefully, drastically improving Developer Experience (DX) and increasing the likelihood of successful onboarding and eventual contribution.

## 📊 Success Metrics
- **Onboarding Success:** 100% of users attempting to run a feature-gated example without the feature flag receive a clear, actionable runtime or compile-time hint rather than a missing symbol error.
- **Support Burden:** Reduction in issues filed complaining about "broken examples" that are actually just missing feature flags.

## 🕵️ Gap Analysis
- **Market View:** High-quality Rust libraries (like `tokio` or `serde`) often use `#[cfg]` attributes in examples to provide a dummy `main` function that simply prints a helpful error message when required features are missing.
- **Our Gap:** We currently expose experimental examples directly without fallback `main` definitions. When the feature flag is missing, the struct definition is omitted from the crate, causing the example's `main` to fail to compile entirely.

## ✅ Acceptance Criteria
- All examples that rely on optional features (e.g., `nova`) must include conditional compilation (`#[cfg(feature = "...")]`) for their actual execution logic.
- All such examples must provide a fallback `main` function (using `#[cfg(not(feature = "..."))]`) that successfully compiles and prints a clear, friendly error message to the user.
- The error message must explicitly state the exact `cargo run` command required to execute the example successfully (e.g., "Error: This example requires the `nova` feature. Please run it with: `cargo run --example story_demo --features nova`").
- The fallback `main` must exit with a non-zero status code to indicate failure to any automated scripts.

## 🚫 Out of Scope
- Modifying the core architecture or making experimental features mandatory.
- Writing a custom Cargo subcommand or intercepting cargo's build process.
- Modifying non-example binaries or core library logic.
