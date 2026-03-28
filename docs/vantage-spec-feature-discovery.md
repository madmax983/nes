# 🔭 Vantage: Spec for Nova Feature Discovery

## 👤 User Story
As an experimental user trying to run a "Nova" feature (like `story_demo`), I want a clear error message and README instructions when I try to run it without the `nova` feature flag enabled, so that I don't get confusing compiler errors about missing structs.

## 💼 Business Problem (So What?)
Compiler errors like "NarrativeGenerator not found" alienate users and prevent adoption of our experimental features. Providing clear, actionable errors and documentation reduces friction, saves time for both users and developers, and encourages exploration of the platform.

## 📈 Success Metrics
- Zero confusing compiler errors when users attempt to run a documented experimental feature without the `nova` feature flag.

## ✅ Acceptance Criteria
- Add a prominent banner or section in `README.md` explicitly stating that running experimental "Nova" features requires appending `--features nova` to the `cargo run` commands.
- Provide a specific, actionable error message in the CLI/terminal if a user attempts to run a binary or feature that relies on `nova` when the feature is disabled (e.g., "Error: This binary requires the 'nova' feature flag. Please re-run with --features nova").

## 🚫 Out of Scope
- Automatically enabling the `nova` feature flag by default.
