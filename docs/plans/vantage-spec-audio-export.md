# 🔭 Vantage: Spec for Audio Export (.wav)

## 👤 User Story
"As a Content Creator or Chiptune Enthusiast, I want to record the emulator's audio output directly to a .wav file, so that I can capture clean sound effects, music tracks, or full gameplay audio without using external capture software."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Capturing clean, lossless audio from retro games is often a cumbersome process involving virtual audio cables or third-party recording software (like OBS). By leveraging the existing `WavExporter` in our core, we can provide a native, one-click solution for high-fidelity audio extraction. This adds immense value for creators, ROM hackers documenting their work, and developers verifying audio emulation accuracy, further solidifying our position as a complete toolkit.

## 📊 Success Metrics
- **Reliability:** The exported .wav file is perfectly synchronized with the gameplay and free of dropped samples or artifacts.
- **Utility:** A user can start and stop recording via a simple UI toggle or hotkey, producing a valid, playable .wav file instantly upon stopping.
- **Adoption:** Used by audio developers and creators for high-quality chiptune extraction.

## 🕵️ Gap Analysis
- **Market View:** Some emulators allow raw audio logging, but it's often a hidden developer feature. Few expose it as a simple "Record Audio" button in the main UI.
- **Our Gap:** The `WavExporter` exists in `nes-core::experimental`, but `nes-desktop` has no interface to feed samples into it or write the resulting file to disk.

## ✅ Acceptance Criteria
- Must provide a toggle in the UI (or a hotkey) to "Start/Stop Audio Recording".
- Must capture the raw 16-bit PCM output from the APU synchronously while recording is active.
- Must format the captured samples into a standard RIFF `.wav` file structure upon stopping the recording.
- Must automatically save the `.wav` file to a designated directory (e.g., `exports/audio/`) with a timestamped or ROM-based filename.
- Must not affect the real-time audio playback through the host system while recording is active.

## 🚫 Out of Scope
- Multi-track recording (e.g., saving Pulse 1, Triangle, and Noise to separate audio channels). This is Phase 2.
- MP3 or OGG encoding (we stick to lossless PCM .wav for Phase 1 to avoid complex dependencies).
