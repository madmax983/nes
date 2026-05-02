# 🔭 Vantage: Spec for Gameplay Video Recording

## 👤 User Story
"As a Content Creator or Speedrunner, I want to easily record high-quality video and audio of my gameplay directly from the emulator, so that I can share my runs and highlights on YouTube, Twitch, or Twitter without needing external capture software like OBS."

## 💼 Business Problem (So What?)
**What business problem does this solve?**
Currently, to share gameplay, users must configure complex third-party software (like OBS Studio) to capture the emulator window, leading to potential frame drops, audio sync issues, or incorrect aspect ratios. By building native video recording directly into the emulator, we reduce friction for creators, guarantee pixel-perfect capture, and encourage the creation of more community content (speedruns, Let's Plays, tutorials) which serves as organic marketing and drives user acquisition.

## 📈 Success Metrics
- **Performance:** Recording 1080p video incurs less than a 5% CPU overhead and maintains a solid 60fps gameplay experience without stuttering.
- **Utility:** A user can hit a hotkey to start recording, play for 10 minutes, and hit the hotkey again to output a standard `.mp4` file that plays perfectly in VLC, YouTube, and Discord.
- **Adoption:** 20% of users who play for more than an hour utilize the recording feature to capture gameplay at least once.

## 🕵️ Gap Analysis
- **Market View:** Top-tier emulators (RetroArch, Dolphin) often have built-in A/V dumping or recording capabilities to ensure deterministic, lag-free captures.
- **Our Gap:** We only support raw TAS movie logging (`.tas.json`) or static audio capture dumps. We lack an accessible, user-facing feature for capturing actual video output (gameplay footage) combined with audio.

## ✅ Acceptance Criteria
- Must provide a hotkey (e.g., F12) or overlay menu option to toggle video recording on and off.
- Must capture the core's native video output (including applied palettes/filters) and synchronized audio into a widely supported format (e.g., `.mp4` with h.264 video and AAC audio).
- Must display a visual indicator (like a red blinking dot in the UI) while recording is active.
- Must ensure the resulting video maintains perfect A/V sync, even if the emulator itself experiences minor frame drops or slowdowns.
- Must save the recorded files to a clearly defined user directory (e.g., `./recordings/`).
- Must handle out-of-disk-space errors gracefully, stopping the recording and notifying the user.

## 🚫 Out of Scope
- Built-in streaming to Twitch or YouTube (RTMP broadcasting is Phase 2).
- Advanced video editing (trimming, text overlays) within the emulator.
- Supporting multiple video codec options out of the box (we will stick to one universally supported standard format for Phase 1).