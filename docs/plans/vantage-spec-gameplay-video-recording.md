# 🔭 Vantage: Spec for Gameplay Video Recording

## 👤 **User Story:**
"As a Content Creator or Speedrunner, I want to easily record high-quality video and audio of my gameplay sessions directly within the emulator, so that I can easily share my runs, tutorials, or highlights without relying on external screen capture software."

## The So What?
**What business problem does this solve?**
Currently, users looking to share their gameplay or create content must use external tools like OBS Studio to capture the emulator window. This introduces unnecessary friction, potential performance issues, and often requires complicated setup to ensure accurate audio and video sync. By providing a built-in gameplay video recording feature, we make our emulator a one-stop-shop for content creation, encouraging users to share their experiences and increasing the visibility and community engagement around our software.

## Metric Definition
- **Success =** The feature can capture at least 60 frames per second (matching the NES framerate) with synchronized audio without dropping frames, resulting in an MP4 file.
- **Adoption =** 20% of users utilize the recording feature at least once a month.
- **Performance =** Gameplay latency increases by less than 5ms while recording is active.

## Gap Analysis
- **Market View:** Top-tier emulators (like RetroArch, FCEUX) offer some form of A/V recording or movie dumping (e.g. AVI dumping).
- **Our Gap:** We currently only support TAS input macro recording (`nes-core::tas`). There is no facility to export the actual graphical and audio output of a session into a standard video format that users can share directly.

## ✅ **Acceptance Criteria:**
- Must provide a UI button/hotkey (e.g., F11) in `nes-desktop` to start and stop gameplay video recording.
- Must capture both the video output (PPU) and the audio output (APU) perfectly synchronized.
- Must encode the output into a widely compatible format (e.g., MP4 or MKV with H.264/AAC encoding).
- Must save the recording to a configurable `recordings/` directory with a timestamped filename.
- Must ensure that starting/stopping the recording does not cause significant emulator lag or desync.
- Must clearly indicate visually to the user when a recording is actively in progress.

## 🚫 **Out of Scope:**
- Live streaming directly to platforms like Twitch or YouTube (Phase 2).
- Advanced video editing capabilities (trimming, adding overlays, etc.).
- Multi-track audio recording (e.g., separating game audio from a user's microphone input).
