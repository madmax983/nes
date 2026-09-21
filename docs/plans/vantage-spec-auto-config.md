# 🔭 Vantage: Spec for Automatic Configuration Generation

## 👤 User Story
"As a New User, I want the emulator to automatically create a default configuration file if one doesn't exist, so that I can immediately launch and play without reading setup instructions or manually copying example files."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, users attempting to launch the emulator for the first time via `cargo run` encounter a hard crash: `failed to read config './nes.toml': No such file or directory`. This is a terrible first impression and creates an immediate onboarding friction point, as highlighted by our UX testing (`ECHO_REPORT.md`). The user is forced to read documentation and manually copy `nes.example.toml` before they can see the product. By automatically generating a default configuration when missing, we ensure a seamless, zero-friction "Time to First Play", reducing user drop-off and frustration.

## 📊 Success Metrics
- **Onboarding Success:** 100% of fresh clones successfully boot to the UI without requiring the user to manually create `nes.toml`.
- **Zero Configuration Friction:** No "File Not Found" errors related to `nes.toml` are presented to first-time users.

## 🕵️ Gap Analysis
- **Market View:** Modern consumer software and mature emulators automatically generate missing configuration files (often in standard user app data directories) upon first launch rather than crashing.
- **Our Gap:** The emulator expects `nes.toml` to exist at the workspace root and strictly fails out when it is missing, rather than initializing a default state.

## ✅ Acceptance Criteria
- When launching `nes-desktop`, `nes-tui`, or any other config-dependent binary, if the target configuration file (`nes.toml` by default) is not found, the application must automatically construct a default configuration object in memory.
- The application must then serialize and write this default configuration to the expected path (e.g., `./nes.toml`).
- The application must log an informational message (not an error) indicating that a default configuration file was created.
- The emulator must continue its boot sequence seamlessly using the newly created configuration, without requiring the user to restart the application.

## 🚫 Out of Scope
- Automatic migration of old configuration file formats to new versions (Phase 2).
- Moving the configuration file from the workspace root to standard OS-specific configuration directories (e.g., `~/.config/` or `%APPDATA%`). For now, we maintain the workspace root expectation but generate the file if missing.
