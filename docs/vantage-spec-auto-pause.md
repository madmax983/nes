# 🔭 Vantage: Spec for Auto-Pause on Focus Loss

## 👤 User Story
As a Desktop Player, I want the emulator to automatically pause when the window loses focus, so that I don't die or miss action while I am looking at another application or monitor.

## 💼 Business Problem (So What?)
Missing out on gameplay due to distractions or managing other windows is a primary source of frustration in desktop gaming. A silent, unprompted death in the game degrades the user's trust and enjoyment. This feature prevents accidental progression and improves the perceived quality of the emulator's desktop experience.

## 📈 Success Metrics
- Zero unwanted gameplay progression when the emulator window is alt-tabbed or clicked away from.
- Feature does not introduce input lag or stutter during normal play.

## ✅ Acceptance Criteria
- The emulator must automatically enter a Paused state immediately upon receiving a `winit` window focus loss event.
- The emulator must automatically resume immediately upon regaining window focus.
- The feature must be configurable and toggleable via `nes.toml` under `[desktop]` as `auto_pause = true` (defaulting to true).
- A visual indication (e.g., a "PAUSED" overlay) must be shown when auto-paused.
- **Critical Caveat:** Auto-pause **must be forcibly disabled** during active Netplay sessions to prevent intentional or unintentional network desyncs and rollbacks.
- **Critical Caveat:** Auto-pause **must be forcibly disabled** during active strict RTA (Speedrun) mode to preserve the integrity of the run timing.

## 🚫 Out of Scope
- Configurable auto-pause delays (e.g., "pause after 5 seconds of inactivity").
- Muting audio specifically during auto-pause (the audio engine should naturally halt if the core is paused).
- Auto-pause support for the Web/Trunk host in this iteration.
