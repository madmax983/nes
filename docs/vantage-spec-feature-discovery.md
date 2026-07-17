# 🔭 Vantage: Spec for Feature Discovery

## 👤 User Story
"As a Developer exploring the repository, I want examples and demos to clearly indicate or automatically handle required feature flags (like `nova`), so that I don't encounter confusing compiler errors about missing types when trying to run them."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
New developers trying out our experimental tools (like `story_demo`) currently hit hard compiler errors (`NarrativeGenerator not found`) because they don't know the `nova` feature is required. This creates a steep learning curve and immediate frustration, reducing the likelihood they will contribute to or adopt our advanced tools. While "adding a banner to the README" is a band-aid, a systemic approach to making examples self-documenting or automatically feature-aware ensures a smooth onboarding experience regardless of which example they run.

## 📊 Success Metrics
- **Onboarding Success:** 0 "not found" compiler errors related to missing features when running documented examples.
- **Developer Experience:** Users are either guided to use the correct feature flag via clear error messages/documentation, or the example runs with the feature automatically enabled.

## 🕵️ Gap Analysis
- **Market View:** Standard Rust practices use `required-features` in `Cargo.toml` for examples, or use `#[cfg(feature = "...")]` with informative `compile_error!` messages.
- **Our Gap:** We currently rely on external documentation (or lack thereof) to inform users about feature requirements for specific binaries or examples, leading to poor DX when those instructions are missed or absent.

## ✅ Acceptance Criteria
- All examples, binaries, or demos that depend on experimental features (e.g., `nova`) must be configured in `Cargo.toml` with `required-features = ["nova"]`.
- When a user attempts to run a feature-dependent example without the feature flag, Cargo should skip it or provide a standard Cargo warning, rather than a cryptic missing type compilation error.
- Alternatively, provide a wrapper script or clear terminal output guiding the user on the exact `cargo run --features nova` command to use.

## 🚫 Out of Scope
- Enabling all experimental features by default for all users.
