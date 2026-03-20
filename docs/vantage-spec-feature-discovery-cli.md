# 🔭 Vantage: Spec for Feature Discovery CLI

## 👤 User Story
As a Developer evaluating or building on the emulator, I want clear, actionable feedback when I attempt to run a tool or demo that requires a disabled Cargo feature (like `nova`), so that I don't waste time debugging compilation errors like "not found".

## 💼 Business Problem (So What?)
Developer experience (DX) is critical for our open-source adoption. Cryptic compiler errors (e.g., `NarrativeGenerator not found`) when trying to run examples like `story_demo` create friction and cause developers to abandon the project. A "Feature Discovery CLI" or clear diagnostic hints directly in the build/run path turns a frustrating dead-end into a helpful, immediate solution.

## 📈 Success Metrics
- Zero developers report "type not found" confusion for examples or binaries gated behind features.
- 100% of attempts to run feature-gated binaries without the feature enabled print a clear, human-readable terminal message explaining exactly which `--features` flag is missing.

## ✅ Acceptance Criteria
- If a user attempts to run a binary or example (e.g., `story_demo`) that relies on an optional workspace feature (e.g., `nova`), the build or runtime must provide a clear error message.
- The message must explicitly state: "This tool requires the '[feature_name]' feature to be enabled. Run with 'cargo run --features [feature_name]'."
- Must not introduce runtime overhead to the core emulator.

## 🚫 Out of Scope
- Auto-enabling features magically without user consent.
- A full interactive terminal UI for selecting build features.
