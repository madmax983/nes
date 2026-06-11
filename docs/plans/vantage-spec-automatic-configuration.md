# 🔭 Vantage: Spec for Automatic Configuration

## 👤 User Story
"As a new User, I want the emulator to work immediately without manual configuration, so that I don't get frustrating 'file not found' errors on my first launch."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, when a new user tries to run the emulator using the default command in our README (`cargo run -p nes-desktop --release -- --config ./nes.toml`), they encounter a fatal error because `nes.toml` does not exist by default. They must manually copy `nes.example.toml` first. This creates immediate onboarding friction and a poor "out of the box" experience. By automating this fallback, we reduce the barrier to entry, decrease early abandonment, and ensure a seamless first impression for new users and contributors.

## 📊 Success Metrics
- **Performance:** Checking for the config file fallback adds negligible time to the startup sequence.
- **Utility:** Launching `cargo run -p nes-desktop --release -- --config ./nes.toml` succeeds out-of-the-box without manual file duplication.
- **Adoption:** 100% of new clones can launch the emulator successfully on the first try.

## 🕵️ Gap Analysis
- **Market View:** Standard developer tools and applications either bundle a default configuration in the binary or automatically generate the required configuration files on their first run.
- **Our Gap:** We require the user to explicitly duplicate an example file before the core commands function, as reported by our internal DX audit (`ECHO_REPORT.md`).

## ✅ Acceptance Criteria
- Must detect if the provided `--config` file path (e.g., `./nes.toml`) is missing.
- Must automatically fall back to reading `./nes.example.toml` if the target config is missing.
- Must print a clear, non-fatal warning to the user indicating that the fallback configuration is being used.
- Must correctly parse and apply the settings from the fallback configuration as if they were provided in the primary config file.

## 🚫 Out of Scope
- A complex graphical configuration wizard or UI dialog.
- Automatically writing or modifying the `nes.example.toml` file.
