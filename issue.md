# 🗣️ Echo: Getting Started example is broken

🤦 **The Confusion:** Tried to run the `SpriteExtractor` example from `crates/nes-core/src/experimental/sprite_extractor.rs`. The compiler said `experimental` could not be found in `nes_core`.

🕵️ **The Reality:** Turns out I needed to enable the `nova` feature in `Cargo.toml`.

💡 **The Fix:** Add a huge banner in the module-level documentation and the doctests saying 'REQUIRES FEATURE NOVA', or ensure the `nes-core` docs explicitly state that the entire `experimental` module is feature-gated.
