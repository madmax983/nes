# 🔭 Vantage: Spec for Feature-Aware Examples

## 👤 User Story
"As a Developer evaluating the emulator's codebase, I want `cargo run` commands for examples and binaries to work out-of-the-box or provide helpful Cargo errors, so that I don't waste time diagnosing `item not found` compiler errors due to missing feature flags."

## 💼 Business Problem (So What?)
**What business problem does this solve?**
Currently, when a user tries to run a feature-gated example or binary (e.g., `story_demo` which requires the `nova` feature), Cargo attempts to compile it with default features. This results in obscure `not found in this scope` errors. The proposed fix from DX feedback is to "add a huge banner in the README," which is a poor solution because documentation gets ignored. By explicitly configuring `required-features` in `Cargo.toml` for binaries and examples, Cargo will automatically skip them or provide a clear, standardized error explaining exactly which feature must be enabled. This transforms a frustrating compiler error into a seamless onboarding experience, saving developers time and reducing support load.

## 📈 Success Metrics
- **Clarity:** 100% of feature-dependent binaries and examples fail with a standard Cargo `required-features` message instead of a compilation error when run without the necessary flags.
- **Engagement:** Reduction in issues reported related to failing examples on fresh clones.

## 🕵️ Gap Analysis
- **Market View:** Standard practice in mature Rust libraries (like `tokio` or `serde`) is to use the `required-features` array in `[[example]]` or `[[bin]]` definitions so that Cargo handles the conditional compilation and user feedback automatically.
- **Our Gap:** We rely on undocumented tribal knowledge or easy-to-miss README notes to communicate which features are required for which binaries, leading to a broken default experience.

## ✅ Acceptance Criteria
- Must identify all binaries (`[[bin]]`) and examples (`[[example]]`) across the workspace that depend on optional features (e.g., `nova`, `mcp-host`, `tas`).
- Must add the `required-features = ["feature_name"]` attribute to their respective definitions in `Cargo.toml`.
- Must verify that running `cargo run --bin <name>` without the feature produces a clean Cargo error stating the feature is required, rather than a Rust compiler error.
- Must *not* break the build when `--all-features` is passed.

## 🚫 Out of Scope
- Enabling experimental features (like `nova`) by default for all users.
- Rewriting the examples to not require the features.
