# 🔭 Vantage: Spec for TAS Ghost Replay

## 👤 User Story
"As a Speedrunner or TAS creator, I want to see a live visual 'ghost' of a previous Tool-Assisted Speedrun (TAS) record overlaying my current gameplay, so that I can practice optimizing my execution and instantly see where I am losing frames."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
While our emulator already supports strict RTA mode (which generates post-run profiles and auto-selects by ROM hash) and TAS movie recording via `nes_core::tas`, players currently have no real-time feedback mechanism to compare their live performance against a pre-recorded TAS file without analyzing the run post-completion. By implementing a live "Ghost" replay feature, we enhance the practice and training capabilities of our emulator, deeply engaging the speedrunning and AI training communities and cementing our position as the ultimate platform for TAS development and execution.

## 📊 Success Metrics
- **Performance:** Rendering the ghost sprite/overlay alongside the live frame must not introduce any emulator slowdown (must maintain stable 60fps).
- **Utility:** A user can load a `.tas.json` file as a ghost and visually distinguish the ghost's position/state from the active player character.
- **Adoption:** 25% of users utilizing RTA or TAS modes activate the Ghost Replay feature during practice sessions.

## 🕵️ Gap Analysis
- **Market View:** Modern racing games heavily utilize "Ghost" features for time trials. Some modified 2D platformers attempt similar features, but they are often buggy or require deep ROM modifications.
- **Our Gap:** We have the deterministic core and TAS movie primitives (`nes_core::tas`) to track frame-by-frame inputs and state, but we lack the PPU visualization pipeline to render a translucent "ghost" layer of the TAS state seamlessly on top of the live player's game state.

## ✅ Acceptance Criteria
- Must provide a CLI flag or UI option (e.g., `--tas-ghost <file.tas.json>`) to load a reference TAS run.
- Must execute the reference TAS in a background instance or maintain its state parallel to the live game.
- Must overlay the sprite/nametable differences or a translucent representation of the ghost's active sprites onto the live PPU output frame.
- Must synchronize the ghost's playback frame with the live game's frame count, pausing the ghost when the live game pauses.

## 🚫 Out of Scope
- Full alpha-blending transparency rendering (if too computationally expensive; a simple flicker or color-tinting effect is acceptable for Phase 1).
- Ghosting for Netplay rollback sessions.
- Multi-ghost support.
