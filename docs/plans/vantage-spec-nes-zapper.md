# 🔭 Vantage: Spec for NES Zapper (Light Gun) Support

## 👤 User Story
"As a retro gaming enthusiast, I want to play classic NES Zapper games using my desktop mouse or touchscreen, so that I can experience iconic titles like Duck Hunt without needing original light gun hardware or a CRT television."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
The NES library includes several iconic titles (e.g., Duck Hunt, Hogan's Alley, Wild Gunman) that fundamentally rely on the Zapper peripheral. Currently, our emulator only supports standard controllers, rendering these games completely unplayable. By supporting the Zapper via standard PC inputs (mouse and touchscreen), we immediately expand the playable library and market appeal of the emulator. Furthermore, supporting advanced peripherals is a strong signal of emulator maturity and comprehensiveness.

## 📊 Success Metrics
- **Playability:** A user can successfully start, play, and complete a round of Duck Hunt using a mouse.
- **Accuracy:** The emulator correctly registers hits on targets as interpreted by the original game logic, without requiring game-specific hacks.
- **Performance:** Polling mouse coordinates and triggering the PPU hit detection logic does not introduce frame drops.

## 🕵️ Gap Analysis
- **Market View:** Nearly all mature NES emulators (Mesen, FCEUX, Nestopia) provide Zapper emulation, typically mapping the crosshair to the mouse cursor and the trigger to the left/right mouse buttons.
- **Our Gap:** We currently have no concept of mouse input in `nes-desktop` or `nes-web`, nor does the `nes-core` have an implementation of the Zapper peripheral logic (which requires specific timing interactions with the PPU to detect light).

## ✅ Acceptance Criteria
- Must implement the hardware logic for the Zapper peripheral in `nes-core` (reading light from the PPU at the specified X/Y coordinate during the frame).
- Must map desktop mouse movement to the Zapper's target coordinates in `nes-desktop`.
- Must map the left mouse button to the Zapper's "half-pull" and "full-pull" (trigger) states.
- Must provide a UI indicator (e.g., a crosshair) when the Zapper is active so the user knows where they are aiming.
- Must allow configuring Port 2 as the Zapper peripheral in the `nes.toml` configuration file.
- Must support touchscreen taps mapping to Zapper shots in the `nes-web` platform.

## 🚫 Out of Scope
- Support for physical light gun replicas (e.g., Sinden Lightgun) via raw USB HID inputs (mouse emulation is sufficient for Phase 1).
- Two-player Zapper support (mapping multiple mice simultaneously).
- Support for other obscure peripherals (e.g., Power Pad, R.O.B.) at this time.
