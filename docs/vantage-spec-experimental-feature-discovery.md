# 🔭 Vantage: Spec for Experimental Feature Discovery

## 👤 User Story
As an early adopter or developer, I want clear indications when a feature or example requires a special compilation flag, so that I can easily enable it instead of encountering confusing missing-item compiler errors.

## 💼 Business Problem (So What?)
When users try to run new or experimental examples (like `story_demo`) without the required `nova` feature flag, they hit raw compiler errors (e.g., `NarrativeGenerator not found`). This causes immediate frustration and abandonment. Clear error handling and documentation reduce onboarding friction and support overhead.

## 📈 Success Metrics
- Decrease in reported confusion around running experimental demos.
- Users correctly apply the `--features nova` flag on their first attempt after reading the documentation.

## 🕵️ Gap Analysis
- Market View: Mature ecosystems (like standard Rust libraries) clearly label conditionally-compiled items and provide helpful errors when an optional dependency is missing.
- Our Gap: We completely hide experimental modules when the `nova` feature is off, causing standard examples to fail opaquely.

## ✅ Acceptance Criteria
- Main `README.md` must include a clear, prominent banner or section explaining that experimental features require the `nova` feature flag.
- Any documentation or quickstart commands for experimental demos (e.g., `story_demo`) must explicitly include the required `--features nova` argument.
- Attempting to build experimental entry points without the feature flag should ideally produce a helpful error message rather than a missing struct error.

## 🚫 Out of Scope
- Enabling the `nova` feature by default.
- Promoting any current `nova` features to stable.
