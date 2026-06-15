# 🔭 Vantage: Spec for Documenting Nova Feature Requirement

## 👤 User Story
"As a Developer evaluating the `story_demo`, I want to know which features I need to enable, so that I don't encounter missing type errors during compilation."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Users attempting to compile or run the `story_demo` hit compilation errors because the `NarrativeGenerator` type is hidden behind the `nova` feature flag, which is not obvious. As reported in `docs/ECHO_NOVA_REPORT.md`, this leads to confusion. A simple banner in the README will immediately unblock users and reduce support requests.

## 📊 Success Metrics
- **Zero Confusion:** Developers attempting to build `story_demo` successfully do so by enabling the `--features nova` flag on their first try.

## 🕵️ Gap Analysis
- **Market View:** Rust projects typically document optional features clearly, especially when they are required for specific examples or demos.
- **Our Gap:** The `story_demo` implicitly depends on a non-default feature, but this is undocumented in the main README.

## ✅ Acceptance Criteria
- Add a prominent banner or note to the README explaining that `story_demo` requires the `nova` feature.
- Provide the exact compilation command (e.g., `cargo run --example story_demo --features nova`).

## 🚫 Out of Scope
- Making `nova` a default feature.
