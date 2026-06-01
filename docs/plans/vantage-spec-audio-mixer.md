# 🔭 Vantage: Spec for Audio Channel Mixer

## 👤 User Story
"As a Chiptune Enthusiast and Homebrew Developer, I want an audio channel mixer with individual volume and mute controls, so that I can isolate specific APU channels (Pulse 1, Pulse 2, Triangle, Noise, DMC) to study their composition or debug my audio engine."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
The NES has a distinct 5-channel APU. Currently, all audio is mixed down into a single master output stream. Musicians, speedrunners, and homebrew developers often need to isolate specific sounds—whether to reverse-engineer a classic track, verify their own game's sound effects, or remove distracting background music to focus on audio cues during gameplay. Providing a native audio mixer allows these specialized users to perform their tasks within our emulator instead of resorting to external tools or ROM hacks, thereby expanding our user base in the enthusiast and creator communities.

## 📊 Success Metrics
- **Performance:** Activating channel isolation or modifying volume does not introduce perceptible audio latency (latency remains < 30ms).
- **Utility:** A user can mute the Triangle and Pulse channels to isolate and listen exclusively to the Noise and DMC channels.
- **Adoption:** 20% of users who load homebrew ROMs interact with the audio mixer at least once per session.

## 🕵️ Gap Analysis
- **Market View:** Specialized audio players (like NSFPlay) and advanced emulators (like Mesen) offer detailed audio debugging and channel mixing.
- **Our Gap:** We provide a master volume and high-quality audio emulation, but no granularity. Users cannot currently inspect or isolate individual APU channels in real-time.

## ✅ Acceptance Criteria
- Must provide UI controls (in `nes-desktop` or `nes-web`) to toggle mute for each of the 5 standard APU channels independently.
- Must provide a master volume slider, as well as individual volume sliders for each channel.
- Must apply volume scaling and muting dynamically without requiring a restart or ROM reload.
- Must accurately mix the modified channels into the final audio output buffer before it reaches the system audio sink.

## 🚫 Out of Scope
- Advanced audio visualization (e.g., piano roll, oscilloscope views, waveform rendering).
- Exporting isolated channels to separate WAV files (stem exporting).
- Support for expansion audio chips (e.g., VRC6, MMC5, Sunsoft 5B) in this initial phase.
