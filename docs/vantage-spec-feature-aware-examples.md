# 🔭 Vantage: Spec for Feature-Aware Examples

## 👤 User Story
As a Developer evaluating the codebase, I want examples to gracefully inform me if I am missing a required feature flag, so that I do not get stuck on cryptic compiler errors like "type not found".

## 💼 Business Problem (So What?)
When developers try to run an example (like `story_demo`) and hit a `NarrativeGenerator not found` compiler error, they assume the codebase is broken. The current UX relies on users manually finding the correct feature flags (`nova`). By making examples feature-aware, we reduce onboarding friction, build trust in our developer experience (DX), and accelerate adoption of advanced features.

## 📈 Success Metrics
- **Zero Cryptic Errors:** Users running an example without required features will never see a generic "not found" compiler error for the main entry point.
- **Actionable Guidance:** Missing feature errors will output the exact `cargo run` command needed to succeed.

## 🕵️ Gap Analysis
- **Market View:** High-quality Rust crates use `#[cfg(feature = "...")]` combined with a fallback `main` function that prints a helpful message if the required features are not active.
- **Our Gap:** Our examples (e.g., `story_demo`) simply fail to compile with missing imports when the user doesn't pass `--features nova`, leading to a frustrating developer experience.

## ✅ Acceptance Criteria
- Examples that require specific features (e.g., `nova`) must use conditional compilation (`#[cfg(feature = "...")]`) to gate the actual logic.
- If the required feature is missing, the example must still compile successfully but output a clear, actionable warning message to the console at runtime.
- The warning message must include the exact command needed to run the example correctly (e.g., `Error: The 'nova' feature is required. Try running: cargo run --example story_demo --features nova`).
- The README should also mention required features for examples where appropriate.

## 🚫 Out of Scope
- Creating a custom cargo runner or wrapper script to automatically inject feature flags.
