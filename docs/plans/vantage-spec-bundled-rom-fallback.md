# Spec: Bundled ROM Fallback

👤 **User Story:** "As a new user, I want the emulator to automatically load a default bundled game when I don't provide a ROM path, so that I can immediately verify the installation works without finding my own files."

❓ **The "So What?" (Business Problem):**
The first-time user experience is currently broken. Running standard commands (e.g., `cargo run -p nes-desktop`) without arguments immediately errors out. This friction increases the drop-off rate for new developers evaluating the codebase. Fixing this ensures a seamless "out of the box" experience, lowering the barrier to entry and improving developer experience (DX).

🎯 **Success Metrics:**
- **Success =** Zero configuration required. `cargo run -p nes-desktop` must load a playable game 100% of the time on a fresh clone.
- **Metric =** "Time to First Playable" (TTFP) drops from minutes (hunting for a ROM) to seconds.

🔍 **Gap Analysis:**
Standard emulators (like Mesen or RetroArch) either open a GUI file picker or display a blank screen when no ROM is provided. Because our emulator is CLI-first and currently lacks a GUI file picker, a blank screen or error offers a poor UX. Auto-loading a legal, bundled homebrew ROM provides immediate proof that the audio/video pipelines are working, exceeding the baseline CLI emulator experience.

✅ **Acceptance Criteria:**
- If the desktop emulator is launched without a ROM path argument, it must fallback to loading `./roms/homebrew/homebrew.nes`.
- The system must print an informative console message indicating the default ROM is being used because no arguments were provided.
- If the bundled ROM is missing, it must fail gracefully with an actionable error message.
- Providing a specific ROM path must override this fallback behavior.

🚫 **Out of Scope:**
- Implementing a GUI file picker.
- Automatically downloading ROMs from the internet.
- Modifying the Web or Netplay startup flows.