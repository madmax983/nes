# 🔭 Vantage: Spec for Custom Keybindings

## 👤 User Story
As a Desktop Player, I want to configure my own keyboard and gamepad mappings, so that I can play comfortably using my preferred input layout rather than the hardcoded defaults.

## ❓ So What?
What business problem does this solve? Currently, users are forced to use hardcoded keyboard layouts (Z/X for A/B) or default gamepad mappings. This alienates players with different keyboard layouts (e.g., AZERTY, Dvorak) or non-standard controllers, reducing the addressable user base and causing early abandonment. By allowing custom keybindings, we increase accessibility, user retention, and emulator usability.

## 📊 Success Metrics
- Success = 0 user complaints regarding hardcoded input mappings.
- 100% of standard gamepad and keyboard inputs can be successfully remapped and persisted.

## 🕵️ Gap Analysis
- Market View: Every modern emulator (RetroArch, Mesen, FCEUX) provides a robust configuration system for input mapping to accommodate diverse setups.
- Our Gap: We currently hardcode inputs and do not expose any user-facing configuration for mapping physical keys to NES buttons, forcing users into a single rigid playstyle.

## ✅ Acceptance Criteria
- Must allow users to map any physical keyboard key to any NES virtual button (A, B, Select, Start, D-Pad).
- Must allow users to map standard gamepad buttons to any NES virtual button.
- The mappings must be saved to and automatically loaded from the user's configuration file (e.g., `nes.toml`).
- Must gracefully fall back to default bindings if the configuration is missing or invalid.

## 🚫 Out of Scope
- Macro recording (e.g., pressing one key to execute a sequence).
- Turbo button configurations (e.g., auto-fire holding).
- In-game UI for rebinding (rebinding via config file is acceptable for Phase 1).
