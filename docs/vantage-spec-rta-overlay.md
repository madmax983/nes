# 🔭 Vantage: Spec for RTA Speedrun Split Overlay

## 👤 User Story
"As a Speedrunner, I want a built-in on-screen timer and split tracking overlay, so that I can monitor my pace against my personal bests."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
We have a robust internal RTA tracking system (managed by `RtaManager`, `CalibrationRecorder`, and `RtaProfile`), but lack a visual overlay to display this data in `nes-desktop`. The user is flying blind during the run. By providing an integrated, accurate on-screen RTA timer and split overlay, we dramatically improve the Developer Experience (DX) and player experience for speedrunners, making our emulator a compelling, out-of-the-box solution for competitive RTA speedrunning.

## 📊 Success Metrics
- **Performance:** Activating the RTA overlay adds < 1ms to frame render time.
- **Utility:** Speedrunners can see their current segment time and delta (ahead/behind) vs the loaded RTA profile in real-time.
- **Adoption:** 80% of sessions launched with RTA enabled utilize the integrated overlay.

## 🕵️ Gap Analysis
- **Market View:** Speedrunners traditionally use external tools which require capturing the emulator window and ensuring the emulator's logic and the timer are synchronized.
- **Our Gap:** We have the deterministic core and built-in RTA segmenting logic, but lack visual feedback.

## ✅ Acceptance Criteria
- Must provide a toggleable UI overlay in `nes-desktop` when RTA is enabled.
- Must display the total run timer, synchronized perfectly with the core's elapsed emulation time.
- Must display a list of upcoming splits (from the active `RtaProfile`), the current split, and previous splits with their time deltas.
- Must automatically update the overlay when the user triggers a split.
- Must highlight the current PB (Personal Best) pace for comparison.

## 🚫 Out of Scope
- Customizable overlay themes and fonts (Phase 2).
- Integration with external services (Phase 2).
