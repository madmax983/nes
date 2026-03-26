# 🔭 Vantage: Spec for Web Mobile Touch Controls

## 👤 User Story
As a Mobile Web Player, I want on-screen touch controls, so that I can play the emulator on my phone or tablet without needing a physical keyboard or bluetooth controller.

## 💼 Business Problem (So What?)
Currently, our web emulator only supports physical keyboard input (via DOM key events). This completely isolates mobile and tablet users who make up a massive segment of web traffic. By adding touch controls, we unlock the mobile market, drastically increasing accessibility, potential user base, and engagement for the web build.

## 📈 Success Metrics
- 100% of NES controller buttons (D-Pad, A, B, Start, Select) are accessible and functional via touch.
- Multi-touch support handles at least two simultaneous inputs (e.g., holding Right and pressing A to run and jump).
- Touch controls do not obstruct the primary gameplay viewing area on standard mobile aspect ratios.
- Input latency from touch event to emulator registration is under 16ms (1 frame).

## 🕵️ Gap Analysis
- Market View: Most modern browser-based retro emulators provide virtual touch pads or on-screen buttons when a mobile user agent is detected or when no physical controller is present.
- Our Gap: The current `web/app.js` and `web/index.html` exclusively listen for and dispatch keyboard events to the Rust WASM core. Mobile users see the game but have no way to interact with it.

## ✅ Acceptance Criteria
- Must render an on-screen visual representation of an NES controller (D-Pad, A, B, Start, Select) when the web emulator is loaded.
- Must bind `touchstart`, `touchend`, and `touchcancel` DOM events to the respective virtual buttons.
- Must translate these touch events into the existing WASM `press_button` and `release_button` calls in `web/app.js`.
- Must support multi-touch (e.g., holding down a D-Pad direction while repeatedly tapping the A button).
- Must automatically hide or disable the touch overlay if the user is on a desktop device (e.g., detect via screen width or absence of touch capability) to avoid visual clutter.
- Must be responsive, scaling the touch areas appropriately for different mobile screen sizes and orientations.

## 🚫 Out of Scope
- Haptic feedback (vibration) on button press.
- Customizable layout or resizable buttons for the touch controls.
- Virtual analog stick (strictly D-Pad for NES).
