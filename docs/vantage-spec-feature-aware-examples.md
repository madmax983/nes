# 🔭 Vantage: Spec for Feature-Aware Examples

## 👤 User Story
"As a Developer evaluating the codebase, I want examples to explicitly require their necessary feature flags, so that I don't encounter confusing compiler errors when trying to run them."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Users attempting to run experimental examples (like `story_demo`) without the required `nova` feature currently encounter obscure `rustc` compilation errors (e.g., `NarrativeGenerator not found`). This poor onboarding experience creates friction and makes the codebase appear broken. By making our examples feature-aware, we provide immediate, actionable feedback to developers, reducing frustration and debugging time.

## 📊 Success Metrics
- **Clear Feedback:** Attempting to run `story_demo` without the `nova` feature results in a clean, informative Cargo error about missing features rather than a raw compiler error.
- **Onboarding Success:** Zero user confusion reports regarding broken experimental examples.

## 🕵️ Gap Analysis
- **Market View:** Mature Rust libraries use Cargo's `required-features` to ensure examples are only built or run when their required dependencies are active.
- **Our Gap:** We do not declare required features on examples like `story_demo`, exposing users to internal compilation details when experimental features are toggled off.

## ✅ Acceptance Criteria
- Examples depending on optional or experimental features (like `story_demo` and `nova`) must declare these dependencies using `required-features`.
- Running `cargo run --example story_demo` without `--features nova` must gracefully halt with a clear feature requirement message, not a missing type compilation error.
- Blanket commands like `cargo test --workspace` or `cargo build --examples` must gracefully skip these examples if the feature is not active.

## 🚫 Out of Scope
- Creating new `nova` examples or expanding the `story_demo`.
