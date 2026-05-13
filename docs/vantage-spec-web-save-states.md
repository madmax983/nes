# 🔭 Vantage: Spec for Web Client Save States

## 👤 User Story
"As a Browser Player, I want to save and load my game state locally in the browser, so that I can resume my progress across different sessions without needing the desktop app."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, the desktop application supports quicksave and quickload via local storage. However, the `nes-web` browser client lacks this functionality. If a user closes their browser tab, their gameplay progress is permanently lost. This severely limits the utility of the web player for long-form RPGs or multi-session games, relegating it to a mere demo tool. By implementing persistent save states in the browser, we achieve feature parity with the desktop client and increase user retention and session length on the web platform.

## 📊 Success Metrics
- **Retention:** 20% increase in returning users launching the same ROM within a 7-day period.
- **Reliability:** 99% of save states written to browser storage are successfully recovered on subsequent loads.

## 🕵️ Gap Analysis
- **Market View:** Other in-browser emulators support saving states to browser storage or exporting them as files.
- **Our Gap:** The emulator core has the capability to generate state snapshots, but the web bridge does not expose UI or shortcuts to trigger these saves, nor does it persist them to browser storage like it does for ROMs.

## ✅ Acceptance Criteria
- Must provide UI controls or keyboard shortcuts in the web interface to trigger state snapshots.
- Must persist the serialized save state to browser storage, uniquely associated with the current game.
- Must automatically restore the most recent save state (if one exists) when a previously played game is reloaded.
- Must handle storage quota limits gracefully by alerting the user instead of panicking.

## 🚫 Out of Scope
- Cloud synchronization of save states across devices.
- Managing multiple save slots per ROM (Phase 1 supports only a single quicksave slot per game).
