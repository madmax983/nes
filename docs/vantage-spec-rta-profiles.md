# 🔭 Vantage: Spec for RTA Profiles

## 👤 User Story:
As a speedrunner, I want to use auto-selected RTA profiles, so that I can automatically have splits and rules configured for the game I'm running.

## ✅ Acceptance Criteria:
- Profiles are defined in `config/rta/profiles/*.toml`.
- Auto-selection determines the profile using the ROM hash.
- Must provide clear feedback on which profile is selected.
- Fallback mechanisms must be well-defined.

## 🚫 Out of Scope:
- Customizing split rules on the fly (Phase 2).
