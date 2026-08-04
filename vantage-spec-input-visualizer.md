# 🔭 Vantage: Spec for Input Visualizer Overlay

## 👤 User Story
"As a Streamer or Speedrunner, I want an on-screen input visualizer overlay, so that my viewers and I can see exactly which controller buttons are being pressed in real-time."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our emulator executes inputs flawlessly and provides TAS recording tools, but there is no native way to display live controller inputs on-screen. Speedrunners and content creators often have to rely on complex, third-party overlay software (like NohBoard) hooked into OBS to show their inputs. By providing a native, zero-setup input visualizer overlay directly within `nes-desktop`, we make our emulator the immediate go-to choice for streaming and speedrunning, increasing our visibility and user adoption in those communities.

## 📊 Success Metrics
- **Performance:** Rendering the input overlay adds less than 1ms of frame time overhead.
- **Utility:** The visualizer accurately reflects state changes on the exact frame the input is polled by the core.
- **Adoption:** 30% of users running the desktop client in RTA (speedrun) mode enable the input visualizer.

## 🕵️ Gap Analysis
- **Market View:** Emulators like RetroArch provide input overlay widgets, and PC gaming streamers heavily use external input display tools.
- **Our Gap:** We process controller inputs efficiently and record them for TAS/AI, but we have no visual representation for the end-user.

## ✅ Acceptance Criteria
- Must provide a toggleable UI overlay in `nes-desktop` that displays a standard NES controller layout.
- Must visually highlight buttons (A, B, D-Pad directions, Start, Select) immediately when pressed.
- Must update the visualization synchronously with the core's polling rate (60Hz) to ensure frame-perfect accuracy.
- Must not interfere with gameplay capture (it should render cleanly on top of the main window).
- Must support hiding or disabling the overlay via a configuration flag or hotkey.

## 🚫 Out of Scope
- Customizable skins or themes for the controller overlay (Phase 2).
- Displaying historical input trails or timelines (TAS timeline is a separate feature).
- Input visualization for the WebAssembly (`nes-web`) or Terminal (`nes-tui`) clients (Phase 1 focuses on `nes-desktop`).
