# 🔭 Vantage: Spec for Visual Input Display Overlay

## 👤 User Story
"As a Speedrunner or Streamer, I want an optional on-screen overlay that displays my active button presses, so that my audience can see my real-time inputs during gameplay and I can verify my own execution."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Our emulator is positioned strongly for systems learning and deterministic behavior (TAS and speedrunning via RTA mode). However, streamers and speedrunners currently have to rely on external, third-party input visualization tools which can desync or add setup friction. By integrating a native Visual Input Display, we reduce friction for content creators and improve the verifiability of speedruns recorded directly from the emulator. This increases the emulator's adoption within the speedrunning community.

## 📊 Success Metrics
- **Adoption:** 20% of users who utilize strict RTA mode enable the input display.
- **Performance:** Rendering the overlay adds less than 1ms per frame overhead to the desktop rendering loop.

## 🕵️ Gap Analysis
- **Market View:** Specialized speedrunning emulators offer native input display toggles to help verify inputs during runs.
- **Our Gap:** We record inputs deterministically and track inputs perfectly for rollback, but we do not expose these active inputs visually to the player during live desktop sessions.

## ✅ Acceptance Criteria
- Must provide a configuration option to toggle the input display overlay in `nes-desktop`.
- Must visually indicate the active state of all NES controller buttons in real-time.
- Must accurately reflect the inputs being sent to the core (post-mapping).
- The overlay must be clearly visible regardless of the background game colors.

## 🚫 Out of Scope
- Customizable skins, colors, or themes for the input display.
- Showing the inputs of remote players during a Netplay session.
