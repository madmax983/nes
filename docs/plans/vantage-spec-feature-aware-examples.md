# 🔭 Vantage: Spec for Feature-Aware Examples

## 👤 User Story
"As a New Contributor, I want examples to automatically specify their required features, so that I can run them successfully without having to memorize or search for the correct `--features` flags."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Running examples like `cargo run --example story_demo` currently fails out of the box because it requires experimental features like `nova`, resulting in confusing compiler errors about missing types (e.g. `NarrativeGenerator not found`). This creates immediate frustration and an onboarding friction point for developers trying to explore the codebase (as noted in `ECHO_NOVA_REPORT.md`). By explicitly declaring required features for examples, Cargo can handle this gracefully, improving the developer experience and saving time.

## 📊 Success Metrics
- **Onboarding Success:** 100% of examples in the workspace can be executed via `cargo run --example <name>` without throwing missing type compiler errors, either running successfully or being skipped gracefully by Cargo with a clear missing-feature message.
- **Zero Configuration Friction:** New contributors do not need to manually read `Cargo.toml` or `README.md` to guess which `--features` flag corresponds to a specific example.

## 🕵️ Gap Analysis
- **Market View:** Standard idiomatic Rust projects utilize Cargo's `required-features` key in the `[[example]]` manifest arrays to prevent un-buildable targets when features are missing.
- **Our Gap:** We introduce experimental APIs behind feature flags (like `nova`), and write examples demonstrating them, but we fail to link the two in our Cargo manifests. This shifts the burden to the user to resolve compilation failures.

## ✅ Acceptance Criteria
- Every example in the workspace that depends on optional code must declare the `required-features = ["feature_name"]` array inside its corresponding `[[example]]` section in `Cargo.toml`.
- Running an example like `story_demo` without providing `--features nova` must no longer result in a compiler error. Instead, Cargo will naturally skip building it or provide an explicit "requires features" warning.
- The change must pass all `cargo test` and `cargo check` CI workflows without regressions.

## 🚫 Out of Scope
- Automatically enabling or defaulting feature flags for main library or binary targets.
- Creating an interactive CLI wrapper to guide users through selecting features.
