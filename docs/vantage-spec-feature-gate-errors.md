# 🔭 Vantage: Spec for Helpful Feature Errors

## 👤 User Story
As a Developer, I want compiler errors for missing experimental features to clearly explain how to enable them, so that I don't waste time searching for missing types like `NarrativeGenerator`.

## 💼 Business Problem (So What?)
Developers evaluating our experimental code (like the `nova` feature) often hit confusing compiler errors (e.g., type not found) if they forget to pass `--features nova`. This creates unnecessary friction and reduces adoption of new features. Clear, actionable error messages improve Developer Experience (DX) and reduce support burden.

## 📈 Success Metrics
- Zero developers report confusion over missing types that are gated behind Cargo features.

## 🕵️ Gap Analysis
- **Market View:** High-quality Rust crates use `#[cfg(feature = "...")]` combined with compile_error! or custom compiler messages to guide users.
- **Our Gap:** We currently just let the compiler fail with a generic "not found" error when a feature-gated type is used but the feature is not enabled.

## ✅ Acceptance Criteria
- When attempting to use experimental binaries or features (like `story_demo` or `NarrativeGenerator`) without the required Cargo feature enabled, the compiler must output a clear error message.
- The error message must explicitly instruct the user to add `--features <feature_name>` to their cargo command.

## 🚫 Out of Scope
- Implementing a custom cargo subcommand.
