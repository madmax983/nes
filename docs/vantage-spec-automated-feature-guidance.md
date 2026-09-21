# 🔭 Vantage: Spec for Automated Feature Guidance

## 👤 User Story
As a developer trying out experimental examples (like `story_demo`), I want the tooling to automatically tell me which cargo features to enable if a required component is missing, so that I don't waste time debugging compilation errors.

## 💼 Business Problem (So What?)
Missing feature flags cause cryptic compilation errors ("Not found") that block new contributors and frustrate users. By providing automated guidance rather than a silent failure, we reduce onboarding friction and improve Developer Experience (DX).

## 📈 Success Metrics
- Zero instances where running a demo missing a required feature results in a generic "Not found" compiler error without a clear hint.

## 🕵️ Gap Analysis
- Market View: Mature libraries (like Tokio or Serde) often use `cfg_if` or custom `compile_error!` messages to clearly state when a feature needs to be enabled.
- Our Gap: Currently, missing features like `nova` result in raw "NarrativeGenerator not found" errors.

## ✅ Acceptance Criteria
- If a user attempts to run a binary or test that requires an optional feature (e.g., `nova`), and the feature is not enabled, the compilation must fail with a clear, human-readable message instructing the user to enable the feature.
- The message must explicitly state the exact command or flag required (e.g., `requires --features nova`).

## 🚫 Out of Scope
- Automatically enabling the features for the user (cargo doesn't support this without complex build scripts).
