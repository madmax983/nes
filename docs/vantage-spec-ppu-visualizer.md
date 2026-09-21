# 🔭 Vantage: Spec for PPU Visualizer

## 👤 User Story
"As a Homebrew Developer or ROM Hacker, I want a real-time PPU pattern table and nametable visualizer, so that I can inspect graphics data, tile loading, and palette mapping as they are rendered by the emulator."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
While our integrated CPU debugger provides deep insight into logic and memory, graphical glitches and rendering issues remain difficult to diagnose without seeing what the PPU is actually processing. Developers often struggle with invisible tiles, incorrect palette assignments, or scroll alignment bugs. By providing a dedicated real-time PPU visualizer, we close the loop on the debugging experience, making our emulator the definitive, comprehensive toolchain for NES graphics development and ROM hacking. This reduces reliance on external tools and keeps users engaged within our ecosystem.

## 📊 Success Metrics
- **Performance:** Activating the PPU visualizer window maintains a steady 60fps when unpaused.
- **Utility:** Developers can easily identify which palette is assigned to a specific background tile in the current nametable.
- **Adoption:** 40% of users who utilize the CPU debugger also open the PPU visualizer during their session.

## 🕵️ Gap Analysis
- **Market View:** Other development-focused emulators (like Mesen) feature robust, real-time PPU inspection tools, including CHR viewers, nametable viewers with scroll boundaries, and sprite (OAM) viewers.
- **Our Gap:** We currently only render the final composite video output to the user. We have the internal state in `nes-core` representing the PPU memory (VRAM, OAM, Palettes), but we do not expose this data visually, making graphical debugging essentially a blind process.

## ✅ Acceptance Criteria
- Must provide a separate UI window or dedicated overlay tab to view real-time PPU state.
- Must display both pattern tables (left and right) with applied palettes.
- Must display the active nametables, including a visual indicator for the current scroll position and screen boundary.
- Must allow selecting a specific tile to view its index, base address, and current palette assignment.
- Must update visually in real-time as the emulator runs, or reflect the exact state when paused via the CPU debugger.

## 🚫 Out of Scope
- Direct manipulation or editing of tile graphics within the visualizer.
- Direct manipulation of OAM (Sprite) data.
- Visualizing sub-frame raster effects (mid-screen split updates) within the nametable viewer (it should reflect end-of-frame state for Phase 1).
