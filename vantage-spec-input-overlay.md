# 🔭 Vantage: Spec for Input Overlay

## 👤 User Story
"As a Player or Streamer, I want a real-time on-screen controller overlay, so that I and my viewers can see exactly which buttons I am pressing during gameplay."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, players and viewers have no visual feedback on controller inputs during gameplay. This makes it difficult for streamers to showcase their technical execution and for players to verify their inputs align with on-screen actions, especially during high-level play or speedruns. Adding an input overlay increases the emulator's utility for content creators and competitive players, driving adoption in those communities.

## 📊 Success Metrics
- **Performance:** Activating the input overlay adds negligible rendering overhead and maintains a steady 60fps.
- **Utility:** The overlay accurately and instantly reflects physical controller inputs with zero perceived delay.
- **Adoption:** 20% of users enable the input overlay during gameplay sessions.

## 🕵️ Gap Analysis
- **Market View:** Many modern emulators and streaming setups feature integrated input displays or support external overlay tools (like NohBoard or gamepadviewer.com) to show controller activity.
- **Our Gap:** We currently do not provide any built-in mechanism to visualize controller inputs. Users must rely on complex third-party setups to achieve this, which creates friction and reduces the appeal of our emulator for streaming and recording.

## ✅ Acceptance Criteria
- Must provide an option in the UI (via `nes-desktop` or `nes-tui`) to toggle an "Input Overlay" on or off.
- Must display a graphical representation of the standard NES controller (D-Pad, A, B, Select, Start).
- Must visually highlight the corresponding buttons on the overlay when they are pressed on the physical controller or keyboard.
- Must update the overlay state in real-time, matching the polling rate of the core emulator input system.
- Must allow basic customization of the overlay position (e.g., top-left, bottom-right).

## 🚫 Out of Scope
- Support for custom controller skins or themes (Phase 2).
- Detailed input history or timeline view (Phase 2).
- Input overlays for netplay peers (local player only for Phase 1).
