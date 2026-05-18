# 🔭 Vantage: Spec for APU Mixer & Visualizer

## 👤 User Story
"As a Homebrew Audio Composer, I want to visualize and independently mute or solo the 5 NES audio channels, so that I can debug music and sound effect interactions in real-time."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, developers and composers can only hear the final composite audio output. If a specific sound effect on the Noise channel is unintentionally interrupting a Pulse channel, or if the DMC channel is behaving erratically, they have no easy way to isolate the problem. By providing a dedicated APU Mixer and Visualizer, we give audio developers precise control to isolate and inspect individual sound channels. This makes our emulator an indispensable tool for NES audio composition and debugging, further solidifying our position as a premier development workstation.

## 📊 Success Metrics
- **Performance:** Activating the APU mixer and visualizer window maintains a steady 60fps without audio stuttering.
- **Utility:** Developers can instantly mute or solo any combination of the 5 audio channels (Pulse 1, Pulse 2, Triangle, Noise, DMC) during active playback.
- **Adoption:** 30% of users utilizing the homebrew debugging tools also engage with the APU Mixer during their session.

## 🕵️ Gap Analysis
- **Market View:** Specialized emulators (like Mesen) feature robust audio mixers that allow users to mute/solo channels, view waveforms, and inspect pitch/volume data.
- **Our Gap:** We currently mix all audio internally into a single output stream and do not expose per-channel volume or state to the user interface, leaving audio debugging as a frustrating, trial-and-error process.

## ✅ Acceptance Criteria
- Must provide a separate UI window or overlay tab (via `nes-desktop`) to view and control APU state.
- Must display 5 separate track controls corresponding to the NES audio channels: Pulse 1, Pulse 2, Triangle, Noise, and DMC.
- Must include toggleable "Mute" and "Solo" buttons for each track.
- Must display a real-time visual indicator (e.g., volume level meter or simple waveform) for the output of each track.
- Must apply mute/solo operations instantly to the audio playback without requiring a restart or pause.

## 🚫 Out of Scope
- Exporting individual channels to separate WAV/PCM files (Phase 2).
- Editing or modifying the instruments, pitch, or sequence data directly via the UI (Read/Mute only for Phase 1).
- Advanced oscilloscope or piano roll visualization (Phase 2).
