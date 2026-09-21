# 🔭 Vantage: Spec for Audio Channel Mixer

## 👤 User Story
"As a Homebrew Composer, I want a visual audio mixer to mute and solo individual APU channels, so that I can isolate and debug specific musical tracks or sound effects."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, developers can visually debug CPU logic and PPU rendering, but they lack tools to isolate audio issues. When composing homebrew tracks or analyzing original games, all 5 channels are mixed together. An Audio Mixer gives sound designers precise control, elevating our toolchain from a purely visual debugger to a comprehensive multimedia workstation, capturing the audio homebrew niche.

## 📊 Success Metrics
- **Performance:** Activating the audio mixer window introduces less than 1ms overhead per frame.
- **Utility:** Developers can completely silence the square waves and noise to hear only the DPCM samples.
- **Adoption:** 20% of users loading the custom `homebrew.nes` utilize the audio mixer during their session.

## 🕵️ Gap Analysis
- **Market View:** Specialized audio trackers (like FamiTracker) have full channel isolation, and emulators like Mesen provide real-time channel toggling and piano rolls.
- **Our Gap:** We mix the APU output immediately, making it impossible to debug missing notes or volume conflicts on a per-channel basis.

## ✅ Acceptance Criteria
- Must provide a UI overlay in `nes-desktop` for the Audio Mixer.
- Must display 5 distinct indicators corresponding to the 5 NES APU channels (Pulse 1, Pulse 2, Triangle, Noise, DPCM).
- Must provide "Mute" and "Solo" toggles for each of the 5 channels.
- Muting a channel must prevent its audio from reaching the final mix without altering emulation accuracy.

## 🚫 Out of Scope
- Piano roll visualizer for note data (Phase 2).
- Exporting isolated tracks to WAV (Phase 2).
- VRC6 or MMC5 expansion audio channels.
