# 🔭 Vantage: Spec for Hitbox Visualizer

## 👤 User Story
"As a Speedrunner or ROM Hacker, I want to see the bounding boxes of active sprites overlaid on the game screen, so that I can visualize collision areas, analyze enemy movement patterns, and verify my route or physics modifications."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Speedrunners rely heavily on precise positioning to manipulate enemy RNG, execute clips, and avoid damage. Without visual aids, they must rely on trial, error, and deep game knowledge to "guess" where hitboxes actually are. By introducing a real-time Hitbox Visualizer that draws bounding boxes directly on the screen, we significantly reduce the barrier to entry for routing and analysis. This positions our emulator as a vital training tool, directly competing with and improving upon the capabilities of existing speedrun-focused tools (like specific LUA scripts for other emulators).

## 📊 Success Metrics
- **Performance:** Activating the hitbox overlay does not drop the frame rate below 60fps.
- **Utility:** Speedrunners can visually identify the boundaries of sprites to aid in routing and glitch execution.
- **Adoption:** 25% of users running in RTA mode enable the hitbox visualizer at least once during a practice session.

## 🕵️ Gap Analysis
- **Market View:** Specialized LUA scripts on emulators like FCEUX or BizHawk are often used by speedrunners to display hitboxes, but these require manual setup per game and are not built-in.
- **Our Gap:** We have the foundational tech to extract sprite data (`experimental/hitbox_visualizer.rs`), but it is not exposed in the user-facing UI, forcing users to rely on external tools or guess sprite boundaries.

## ✅ Acceptance Criteria
- Must provide a toggleable option in the UI (e.g., overlay menu or hotkey) to enable the Hitbox Visualizer.
- Must render 8x8 hollow bounding boxes (or appropriate size for 8x16 mode if supported later) around active sprites.
- Must overlay these bounding boxes directly on top of the rendered gameplay footage in real-time.
- Must update the positions and visibility of the hitboxes every frame based on the OAM (Object Attribute Memory) state.
- Must *not* interfere with or be captured by TAS movie recording (it is a display overlay, not game state).

## 🚫 Out of Scope
- Game-specific, pixel-perfect internal hitboxes (this requires game-specific reverse engineering; Phase 1 only uses the hardware sprite boundaries from OAM).
- Modifying sprite positions via the visualizer UI.
- Visualizing background/tile collision data (Phase 2, requires game logic understanding).
