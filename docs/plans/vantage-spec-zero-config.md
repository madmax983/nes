# 🔭 Vantage: Spec for Zero-Config Startup

## 👤 User Story
"As a first-time User, I want the emulator to start immediately without manually creating a configuration file, so that I can experience the software instantly."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, first-time users experience a fatal error (`failed to read config './nes.toml': No such file or directory`) because the default configuration file (`nes.toml`) is not created automatically. Users are forced to read the documentation and manually run `cp nes.example.toml nes.toml` before the emulator works. This friction leads to a poor onboarding experience and potential user drop-off. By automating the creation of default configurations, we eliminate "Time to First Play" barriers.

## 📊 Success Metrics
- **Utility:** 100% of fresh checkouts successfully launch `nes-desktop` without requiring manual file copy steps.
- **Adoption:** "Getting Started" documentation is simplified, completely removing the manual configuration step.

## 🕵️ Gap Analysis
- **Market View:** Modern software, particularly emulators and CLI tools, automatically generate default configurations in the appropriate user directories if they do not exist.
- **Our Gap:** The emulator strictly requires a configuration file but provides no built-in mechanism to bootstrap one from internal defaults or example templates.

## ✅ Acceptance Criteria
- When launching `nes-desktop` or `nes-tui`, if `nes.toml` (or the specified config path) does not exist, the application must automatically generate it using the default settings found in `nes.example.toml`.
- A log message should notify the user that a default configuration was created.
- The emulator must successfully proceed to launch after generating the config, rather than exiting.

## 🚫 Out of Scope
- GUI-based configuration editors.
- Migrating existing configuration formats.
