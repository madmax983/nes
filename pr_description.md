* 👤 **User Story:** "As a Developer evaluating the emulator's codebase, I want `cargo run` commands for examples and binaries to work out-of-the-box or provide helpful Cargo errors, so that I don't waste time diagnosing `item not found` compiler errors due to missing feature flags."
* 💼 **Business Problem (So What?):** By explicitly configuring `required-features` in `Cargo.toml` for binaries and examples, Cargo will automatically skip them or provide a clear, standardized error explaining exactly which feature must be enabled. This transforms a frustrating compiler error into a seamless onboarding experience, saving developers time and reducing support load.
* 📈 **Success Metrics:** 100% of feature-dependent binaries and examples fail with a standard Cargo `required-features` message instead of a compilation error when run without the necessary flags.
* 🕵️ **Gap Analysis:** We currently rely on undocumented tribal knowledge or easy-to-miss README notes to communicate which features are required for which binaries, leading to a broken default experience compared to standard practice in mature Rust libraries.
* ✅ **Acceptance Criteria:**
  - Must identify all binaries and examples across the workspace that depend on optional features.
  - Must add the `required-features` attribute to their respective definitions in `Cargo.toml`.
  - Must verify that running `cargo run --bin <name>` without the feature produces a clean Cargo error.
* 🚫 **Out of Scope:** Enabling experimental features (like `nova`) by default for all users, or rewriting the examples to not require the features.
