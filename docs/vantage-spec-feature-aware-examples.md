# 🔭 Vantage: Spec for Feature-Aware Examples

## 👤 User Story
As a Developer trying out the emulator, I want the build system to clearly inform me if I am missing a required feature flag when running an example, so that I don't waste time debugging confusing "not found" compiler errors for conditionally compiled code.

## 💼 Business Problem (So What?)
Currently, if a user attempts to run a feature-gated example (such as `story_demo` which requires the `nova` feature) without specifying the flag, the compiler fails with generic and confusing errors like `NarrativeGenerator not found`. This creates a terrible first impression, wastes developer time, and introduces high friction for onboarding. By providing feature-aware examples that explicitly state their requirements, we reduce developer frustration, lower the barrier to entry, and improve the overall developer experience.

## 📈 Success Metrics
- **Zero Confusion:** 0 new bug reports or user complaints related to missing types for feature-gated examples.
- **Time to Resolution:** Developers immediately understand what flag is missing and apply it without having to search through source code or documentation.

## 🕵️ Gap Analysis
- **Market View:** In the Rust ecosystem, it is a standard practice to utilize the `required-features` key in `Cargo.toml` for `[[example]]` targets, or to use `compile_error!` with `cfg` attributes, to clearly communicate feature requirements.
- **Our Gap:** We rely on implicit knowledge. Our feature-gated examples simply fail to compile with confusing missing symbol errors if the correct flag is not provided, rather than giving a clear, actionable message.

## ✅ Acceptance Criteria
- All examples in the workspace that rely on specific feature flags (e.g., `nova`) must define `required-features = ["<feature_name>"]` in their respective `[[example]]` blocks in `Cargo.toml`.
- Any demo scripts or documentation referencing these examples must clearly state the required feature flags.
- If an example cannot use `required-features`, it must include a `compile_error!` macro that explicitly alerts the user to enable the required feature.

## 🚫 Out of Scope
- Automatic enabling of feature flags via custom CLI wrappers.
- Restructuring the workspace dependency graph to eliminate the need for feature flags.
