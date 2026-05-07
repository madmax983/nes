# 🔭 Vantage: Spec for APU Visualizer

## 👤 User Story
"As a Homebrew Audio Composer, I want a real-time APU piano roll and channel visualizer, so that I can see active notes, debug sound channels, and tune my audio engine while the game runs."

## 💼 Business Problem (So What?)
**What business problem does this solve?**
While our emulator provides deterministic execution, audio debugging is completely opaque. Composers must rely on external trackers and guess how their audio engine translates to the APU at runtime. By offering a visual APU piano roll and channel state inspector, we become a complete suite for homebrew development. This prevents developers from abandoning our platform for competitors when debugging sound.

## 📈 Success Metrics
- **Performance:** Activating the APU visualizer window must not cause audio stuttering or drop emulator framerate below 60fps.
- **Utility:** Developers can isolate and mute individual APU channels in real-time.
- **Adoption:** 30% of users loading the custom homebrew ROM utilize the APU visualizer during their session.

## 🕵️ The Reality:
- **Market View:** Class-leading development emulators feature integrated piano rolls, oscilloscope views, and channel toggles.
- **Our Gap:** We currently render mixed audio via nes-desktop and nes-web, but provide zero insight into the active state of the 5 individual APU channels. Users have no way to see what pitch, volume, or duty cycle is currently playing.

## ✅ Acceptance Criteria
- Must provide a dedicated UI window or overlay tab for APU state visualization.
- Must display real-time frequency, volume, and duty cycle for Pulse 1, Pulse 2, Triangle, Noise, and DPCM channels.
- Must display a scrolling piano roll representing active notes on tonal channels.
- Must allow users to mute/unmute individual channels during gameplay.
- Must visually reflect APU register writes dynamically as the emulator runs.

## 🚫 Out of Scope
- Editing or injecting audio data directly (Phase 1 is read-only).
- Exporting isolated channel audio to WAV/NSF (Phase 2).
- Advanced oscilloscope waveform drawing (Phase 2).
