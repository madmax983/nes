# 🔭 Vantage: Spec for WAV Audio Recording UI

## 👤 User Story
"As a Player or Content Creator, I want to record the emulator's audio output directly to a standard .wav file, so that I can capture chiptunes, sound effects, or gameplay audio for use in videos and remixes without using external recording software."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
While the emulator possesses experimental internal support for exporting raw audio, there is currently no user-facing way to trigger or manage this feature. Users are forced to rely on complex external tools like OBS or virtual audio cables to capture emulator audio. By integrating a seamless, one-click recording UI directly into the emulator, we enhance the content creation experience, empowering the chiptune and speedrunning communities to easily extract high-quality audio directly from the source. This increases the value proposition of our emulator as a creator-friendly platform.

## 📊 Success Metrics
- **Performance:** Activating the audio recording feature causes zero noticeable frame drops or audio stuttering during gameplay.
- **Utility:** A user can initiate, stop, and successfully save a .wav file entirely through the emulator's user interface.
- **Adoption:** 20% of users who utilize TAS playback or Time Machine rewinds also use the audio recording feature to capture their sessions.

## 🕵️ Gap Analysis
- **Market View:** Feature-rich emulators often include built-in A/V recording capabilities, making it trivial for users to export their gameplay without third-party software.
- **Our Gap:** We already have the backend logic, but it is hidden and lacks any integration with the frontend. There are no menus, hotkeys, or indicators to manage audio recording.

## ✅ Acceptance Criteria
- Must add a "Record Audio" option to the menu and/or a dedicated hotkey to start/stop recording.
- Must display a visual indicator (e.g., a "Recording" icon or text in the overlay) while audio is actively being captured.
- Must automatically stream audio chunks without blocking the main emulation loop.
- Must prompt the user for a save location (or use a sensible default like `./recordings/`) when recording is stopped, and save a valid, playable `.wav` file.
- Must ensure the `.wav` file correctly reflects the audio output generated during the recorded segment, including correct sample rate and format.

## 🚫 Out of Scope
- Video recording or full A/V export (e.g., `.mp4` or `.mkv` generation).
- Multi-track audio recording (separating individual APU channels into different tracks).
- In-emulator audio editing or playback.
