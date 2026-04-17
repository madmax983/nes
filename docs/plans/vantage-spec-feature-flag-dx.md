# 🔭 Vantage: Spec for Feature Flag DX

## 👤 User Story
As a Developer attempting to run experimental examples (like `story_demo`), I want clear, immediate feedback if a required feature flag (e.g., `nova`) is missing, so that I don't waste time debugging opaque compiler errors like "Module or type not found".

## 💼 Business Problem (So What?)
We are shipping advanced R&D features behind `#[cfg(feature = "nova")]` flags to keep the core stable. However, our documentation and examples do not always communicate these prerequisites effectively. When developers encounter cryptic compiler errors on their first run, it damages the perceived stability of the project and increases Time to First Meaningful Run (TTFMR), leading to abandonment.

## 📈 Success Metrics
- Zero confused developer reports regarding missing features when running documented examples.
- All code snippets in `README.md` that require non-default features must explicitly mention the required flag in the snippet.

## 🕵️ Gap Analysis
- **Market View:** Ecosystems like `tokio` or `serde` heavily utilize feature flags. When a feature is missing, they provide helpful compiler hints or prominently display badges/banners in their documentation indicating the required flag.
- **Our Gap:** We rely on developers manually discovering `#[cfg]` flags in the source code when they encounter an error. We lack clear documentation and actionable compiler diagnostics.

## ✅ Acceptance Criteria
- `README.md` must include a highly visible banner or warning note in the "Getting Started" or examples section explicitly stating that experimental features like `story_demo` require the `nova` feature to be enabled.
- Any commands provided in the documentation to run `nova` features must include `--features nova`.
- (Bonus) Investigate adding `#[cfg(feature = "nova")]` stubs that `compile_error!` with a helpful message when the feature is missing, rather than simply hiding the module.

## 🚫 Out of Scope
- Removing the `nova` feature flag entirely and merging experimental code into the core.
- Enabling `nova` by default in `Cargo.toml`.
