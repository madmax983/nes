# 🔭 Vantage: Spec for Auto-Config Generation

## 👤 User Story
"As a First-Time User, I want the emulator to automatically create a default configuration file if one is missing, so that I can immediately launch and play without reading setup instructions or encountering file-not-found errors."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, new users encounter a fatal `os error 2` (`failed to read config './nes.toml'`) immediately upon trying the default launch command. This creates high friction during the critical "First Time User Experience" (FTUE). A manual step to `cp nes.example.toml nes.toml` is an unnecessary barrier to entry. By automatically scaffolding a default configuration, we ensure a seamless "Out-of-the-Box" experience, dramatically reducing bounce rates for first-time evaluators and potential contributors.

## 📊 Success Metrics
- **Friction Reduction:** 0% of users encounter the `failed to read config './nes.toml'` crash on first launch.
- **Time to First Frame:** Time spent from initial `cargo run` to seeing the emulator window drops from an average of 2 minutes (reading docs, copying files) to ~10 seconds.

## 🕵️ Gap Analysis
- **Market View:** Modern consumer software and developer tools (like VS Code, Cargo itself) never crash on missing default configs; they silently scaffold defaults or fall back to in-memory defaults while explicitly notifying the user.
- **Our Gap:** We strictly demand a user-provided file and crash hard if it is absent, breaking the principle of least astonishment.

## ✅ Acceptance Criteria
- Must detect if the configured config file path (default `./nes.toml`) is missing.
- Must automatically generate a default configuration file at that location based on an embedded default template (or copying `nes.example.toml` if bundled).
- Must print a helpful info-level log message stating that a default configuration file was created.
- Must immediately proceed to launch the emulator using the newly created configuration without requiring a restart.

## 🚫 Out of Scope
- Interactive CLI wizards to customize the configuration during first launch.
- Migrating configurations from older emulator versions.
