# 🔭 Vantage: Spec for Nova Discoverability

## 👤 User Story
As a Developer exploring the repository, I want clear instructions or automated fallbacks when running experimental demos like `story_demo`, so that I don't encounter confusing build errors about missing components.

## ❓ So What? (Business Problem)
**What business problem does this solve?**
New contributors and users trying out the repository often attempt to run examples or demos they see mentioned in the documentation or codebase. Currently, running `story_demo` without the `nova` feature flag results in a confusing "NarrativeGenerator not found" compiler error. This creates immediate friction, degrades the Developer Experience (DX), and prevents users from experiencing the value of our experimental features. By making these features discoverable or self-correcting, we reduce onboarding time and improve contributor retention.

## 📊 Success Metrics
- **Onboarding Success:** 100% of users attempting to run `story_demo` either successfully launch it or receive a clear, actionable error message instructing them to use `--features nova`.
- **Zero Confusion:** No users report "missing component" errors for gated features without an obvious path to resolution.

## 🕵️ Gap Analysis
- **Market View:** Modern toolchains (like Cargo itself) often suggest missing feature flags when an import fails.
- **Our Gap:** We rely on the user to magically know that `story_demo` requires the `nova` feature, which is not communicated at the point of failure.

## ✅ Acceptance Criteria
- Must provide a descriptive compiler error (e.g., via `#[cfg(not(feature = "nova"))] compile_error!(...)`) if a user attempts to run `story_demo` without the required flag, OR provide a wrapper script that automatically adds the `--features nova` flag.
- Must successfully guide the user to run the demo without confusion.

## 🚫 Out of Scope
- Making `nova` features part of the default build (experimental features should remain opt-in to keep the core binary small).
