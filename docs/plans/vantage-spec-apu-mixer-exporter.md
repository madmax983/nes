# Spec: APU Channel Mixer & WAV Exporter

## 👤 User Story
As a Chiptune Artist or Content Creator, I want to isolate, mute, and individually export the NES APU audio channels (Pulse 1, Pulse 2, Triangle, Noise, DPCM), so that I can sample raw retro sounds and remix them in my DAW.

## 💼 Business Problem (So What?)
Currently, emulators provide a single mixed audio output. Creators looking to sample specific sound effects or music stems must resort to complex ROM hacking or searching for pre-ripped stems. By providing built-in channel isolation and export, we expand our user base from just gamers and developers to audio producers and content creators, increasing the utility and visibility of our tool in the creative market.

## 📈 Success Metrics
- **Adoption:** 5% of monthly active users utilizing the export feature.
- **Accuracy:** Exported audio matches standard NES APU output exactly.
- **Performance:** Audio export and channel muting does not increase emulator CPU load by more than 2%.

## 🕵️ The Reality (Gap Analysis)
- **Market:** Tools like Mesen have debuggers with channel muting, but seamless, high-quality multitrack WAV exporting is clunky or requires secondary recording software.
- **Our Status:** We have a highly accurate deterministic emulator and we are already capturing reference PCM audio for testing. We have the foundation but lack the user-facing UI and direct-to-disk multitrack WAV export.

## ✅ Acceptance Criteria
- Users can toggle mute/solo for each of the 5 standard APU channels via the Desktop/Web UI.
- Users can click "Record Audio" to begin capturing the audio output.
- When recording is stopped, the emulator saves a standard `.wav` file to the user's disk.
- Option to export a single mixed `.wav` or a "Multitrack" export which saves 5 separate `.wav` files simultaneously.
- Muting a channel must silence it immediately without causing pops or clicks in the live audio output.

## 🚫 Out of Scope
- Support for expansion audio channels (VRC6, Sunsoft 5B, etc.) in Phase 1.
- Built-in effects processing (reverb, EQ).
- MIDI output or synthesis replacement.
