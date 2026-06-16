# 🔭 Vantage: Spec for Nova Feature Discoverability

## 👤 User Story
"As an early adopter or developer exploring the repository, I want to clearly understand which experimental features, tools, and demos are gated behind the `nova` feature flag, so that I don't waste time troubleshooting cryptic compiler errors like 'NarrativeGenerator not found' when trying to run demo examples."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our experimental R&D features, visualizers, and tools are hidden behind the `nova` feature flag to keep the core emulator footprint small. However, our documentation (or user expectations based on the repo contents) points to demos like `story_demo` that fail to compile out of the box. This creates immediate friction for our most engaged users—the early adopters and developers wanting to explore our cutting-edge capabilities. A failed build due to an uncommunicated feature flag is a bad Developer Experience (DX) that breaks trust. By making the `nova` requirement explicit, we reduce developer frustration, decrease "issue tracker noise" regarding broken demos, and smooth the onboarding process for contributors interested in our R&D work.

## 📊 Success Metrics
- **Onboarding Friction:** Zero user reports or questions about missing experimental structs/modules (like `NarrativeGenerator`) when running examples.
- **Discoverability:** The `README.md` explicitly calls out the `nova` feature flag and provides the exact command needed to run experimental demos.

## 🕵️ Gap Analysis
- **Current State:** The `ECHO_NOVA_REPORT.md` indicates users try to run experimental demos and hit a hard compiler error because the required feature is not enabled by default, and there is no visible instruction on how to enable it.
- **Our Gap:** There is a disconnect between the code we offer (which requires `--features nova`) and the instructions provided to the user. We assume users know to look at `Cargo.toml` or source code `#[cfg(feature = "nova")]` attributes, which violates good DX principles.

## ✅ Acceptance Criteria
- A prominent section must be added to the main `README.md` dedicated to "Experimental Features (Nova)".
- This section must explicitly list some of the key features gated by `nova` (e.g., `story_demo`, visualizers, `NarrativeGenerator`).
- The documentation must provide exact, copy-pasteable Cargo commands demonstrating how to run a demo with the feature enabled (e.g., `cargo run --features nova --bin story_demo`).
- Any individual `README.md` files or guides specifically discussing these experimental features must also state that `--features nova` is required.

## 🚫 Out of Scope
- Removing the `nova` feature flag to make experimental features default (we intentionally keep the base footprint small).
- Modifying the Rust code to produce a friendlier compiler error (this is constrained by the Rust compiler's behavior with `#[cfg]`).
