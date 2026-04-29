# 🔭 Vantage: Spec for Feature Flag DX

## 👤 User Story
"As a New User or Contributor exploring experimental features (like the `story_demo`), I want explicit compiler errors or clear terminal messages if I attempt to run code that requires a specific feature flag, so that I don't waste time debugging cryptic 'not found' errors."

## 💼 Business Problem (So What?)
Cryptic compiler errors (e.g., `NarrativeGenerator not found`) when attempting to run documented demos or tests create immediate friction and make the codebase feel broken. This violates the "Time to First Play" principle. By implementing a robust Developer Experience (DX) for feature flags, we reduce onboarding support requests, stop users from giving up early, and clearly communicate the boundaries of experimental (`nova`) features versus core stability.

## 📈 Success Metrics
- **Zero Cryptic Errors:** Users attempting to run feature-gated code without the flag will receive a direct, actionable `compile_error!` or equivalent terminal message rather than a cascade of "struct not found" errors.
- **Onboarding Speed:** Reduced time spent searching for missing imports or checking the issue tracker when running `story_demo` or similar targets.

## 🕵️ Gap Analysis
- **Market View:** High-quality Rust projects use `#![cfg(feature = "...")]` combined with `compile_error!` or user-friendly panic messages in binaries to guide users to the correct `cargo` invocation.
- **Our Gap:** As highlighted in `ECHO_NOVA_REPORT.md`, our experimental features (like those gated behind the `nova` feature) simply disappear from the module tree when disabled. When a user tries to run a demo binary that depends on them, the compiler complains about missing symbols, rather than explaining *why* they are missing (the lack of the `--features nova` flag).

## ✅ Acceptance Criteria
- Any binary, test, or example target that *strictly requires* an optional feature (e.g., `nova`) must include a top-level check.
- If the required feature is not enabled, the build should fail with an explicit `compile_error!("This target requires the 'nova' feature. Run with --features nova");` OR the binary should compile a stub `main` that immediately prints a user-friendly error and exits.
- The `README.md` must clearly denote which launch commands or examples require the `nova` feature flag with a prominent banner or note.
- This pattern must be applied to the `story_demo` target specifically identified in the Echo report, and established as a standard for all future feature-gated targets.

## 🚫 Out of Scope
- Removing the `nova` feature flag entirely. Experimental boundaries are still required.
- Automatically modifying the user's `Cargo.toml` or global Cargo config to default the feature on.
