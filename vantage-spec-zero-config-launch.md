# 🔭 Vantage: Spec for Zero-Config Launch

## 👤 User Story
"As a New User, I want to run the emulator immediately without manual setup, so that I can experience the product's value instantly."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, users experience friction upon first run. They encounter an `os error 2` because `nes.toml` does not exist, requiring them to manually copy `nes.example.toml` and locate a ROM. By implementing a zero-config fallback that automatically loads a bundled homebrew ROM and default settings, we eliminate onboarding friction, reduce support requests, and increase early adoption rates.

## 📊 Success Metrics
- **Time to First Frame:** New users see a running game within 5 seconds of the first `cargo run` without any manual file operations.
- **Error Rate:** Drop the `os error 2` configuration missing error rate to 0% for first-time launches without arguments.

## 🕵️ Gap Analysis
- **Market View:** Modern consumer software and developer tools "just work" out of the box with sensible defaults (e.g., sensible default configs, starter templates).
- **Our Gap:** We require explicit configuration and ROM paths via CLI arguments or manual file copying before the core value is demonstrated.

## ✅ Acceptance Criteria
- If no `nes.toml` exists and no CLI arguments are provided, the emulator must automatically start using default configuration values.
- The emulator must automatically load the bundled `roms/homebrew/homebrew.nes` if no ROM path is provided.
- Must log an informative message to the console explaining that default settings and the bundled ROM are being used.

## 🚫 Out of Scope
- Automatic downloading of commercial ROMs.
- A GUI configuration wizard on first launch (Phase 2).
