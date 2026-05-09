# 🔭 Vantage: Spec for APU Visualizer

## 👤 User Story
"As a Homebrew Developer or Audio Composer, I want a real-time APU channel visualizer, so that I can inspect audio registers, view active wave channels, and debug sound engine logic directly within the emulator."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
We already provide powerful debugging tools for the CPU and PPU, but audio debugging remains a black box. Composers and developers often struggle to figure out why a sound effect isn't playing, or why a channel is unexpectedly silenced. By providing a real-time APU visualizer, we make our emulator a complete suite for homebrew development, enabling audio engineers to diagnose issues without relying on external tools.

## 📊 Success Metrics
- **Performance:** Rendering the APU visualizer adds negligible overhead and maintains 60fps.
- **Utility:** Developers can see real-time volume levels, pitch, and duty cycle for all 5 standard NES channels.
- **Adoption:** 30% of users who utilize the debugging suite open the APU visualizer when diagnosing audio issues.

## 🕵️ Gap Analysis
- **Market View:** Specialized emulators provide a "Piano Roll" or oscilloscope view for audio channels, which is highly valued by composers.
- **Our Gap:** We currently only output the final mixed audio buffer. We have the internal state of the 5 APU channels in `nes-core`, but do not expose this state visually.

## ✅ Acceptance Criteria
- Must provide a separate UI window or overlay tab to view real-time APU state.
- Must display the current status, volume, and period for Pulse 1, Pulse 2, Triangle, Noise, and DPCM channels.
- Must include a basic oscilloscope or volume meter for each channel to visually represent its current output.
- Must update visually in real-time as the emulator runs, or reflect the exact state when paused.

## 🚫 Out of Scope
- Full "Piano Roll" note history visualization (Phase 2).
- Exporting channel data to external audio files (Phase 2).
- Support for expansion audio chips (e.g., VRC6) for Phase 1.
