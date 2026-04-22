# 🔭 Vantage: Spec for Zapper (Light Gun) Support

## 👤 User Story
"As a Retro Gamer, I want to play classic light gun games (like Duck Hunt or Wild Gunman) using my mouse or touchscreen as a Zapper, so that I can experience the full library of NES titles."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
While our emulator provides excellent support for standard controller input, we are locking users out of a highly memorable and popular segment of the NES library: Zapper games. This lack of peripheral support reduces overall user engagement and prevents our emulator from being considered a "complete" retro gaming platform. By adding Zapper support utilizing existing mouse/touch interfaces, we immediately unlock these classic titles, expanding our market appeal and competitive parity with established emulators like FCEUX or Mesen.

## 📊 Success Metrics
- **Compatibility:** Games like *Duck Hunt* correctly register hits and misses when using the Zapper input.
- **Performance:** Polling Zapper input adds negligible overhead to the core emulation loop.
- **Usability:** Users can intuitively toggle Zapper mode without editing configuration files.

## 🕵️ Gap Analysis
- **Market View:** Other major emulators seamlessly support translating mouse clicks to Zapper trigger pulls and mouse position to screen coordinates.
- **Our Gap:** We currently only simulate standard controllers (`Player 1` and `Player 2` gamepads). We have no mechanism in `nes-core` for the NES to read Zapper states (trigger pull, light sensor), nor do we have UI hooks in `nes-desktop` or `nes-web` to capture mouse events for this purpose.

## ✅ Acceptance Criteria
- Must introduce a Zapper input interface to `nes-core` that can supply the state of the trigger and light sensor on demand.
- Must accurately simulate the NES Zapper hardware behavior (e.g., pulling the trigger signals the game, and the game subsequently checks the light sensor during the white-flash frames).
- Must provide a toggle in the UI (or via hotkey/CLI) to assign the Zapper to Controller Port 2.
- Must capture mouse movement and left-clicks in `nes-desktop` and `nes-web` and translate them to screen coordinates and trigger actions for the Zapper simulation.
- Must support visual crosshairs when Zapper mode is active to help players aim using their mouse.

## 🚫 Out of Scope
- Support for other peripherals (e.g., Power Pad, R.O.B., Arkanoid Controller) - Phase 2.
- Physical light gun hardware integration (e.g., Sinden Lightgun) beyond standard mouse emulation.
