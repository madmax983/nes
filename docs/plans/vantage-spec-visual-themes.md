# 🔭 Vantage: Spec for Visual Themes

## 👤 User Story
"As a Player, I want to apply post-processing visual filters (like Gameboy green or Sepia) to the emulator screen, so that I can experience classic games with different nostalgic or high-contrast aesthetics."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
While accuracy is our primary goal, casual users and retro enthusiasts often seek a specific "vibe" or nostalgic aesthetic that pure composite output doesn't provide. Competitors offer shaders, LCD grids, and color palette swaps. By exposing the existing `ThemeFilter` (Gameboy, Sepia, VirtualBoy, etc.) as user-selectable options, we increase the casual appeal and shareability of the emulator, attracting users who enjoy customizing their retro gaming experience.

## 📊 Success Metrics
- **Performance:** Applying a visual theme adds less than 1ms overhead to the frame rendering time on average hardware.
- **Utility:** Users can swap between themes instantly via the UI without reloading the ROM or losing state.
- **Adoption:** 15% of casual players toggle a visual theme at least once.

## 🕵️ Gap Analysis
- **Market View:** Nearly all modern emulators (RetroArch, OpenEmu) support robust shader pipelines or built-in visual filters to simulate different hardware or aesthetics.
- **Our Gap:** The `nes-core` has an experimental `ThemeFilter` capable of post-processing the framebuffer (Grayscale, Gameboy, Sepia, VirtualBoy), but this functionality is not exposed to the user in `nes-desktop` or `nes-tui` through any menu or configuration option.

## ✅ Acceptance Criteria
- Must provide a dropdown or cycle button in the UI overlay to select a "Visual Theme".
- Must support at least the existing themes: Default (None), Grayscale, Gameboy, Sepia, and VirtualBoy.
- Must apply the selected theme to the raw framebuffer before it is presented to the user.
- Must persist the user's theme choice across emulator restarts (e.g., save in `nes.toml` under `[ui] visual_theme`).
- Must not affect the underlying saved state or TAS recordings (the theme is a display-only post-process).

## 🚫 Out of Scope
- Custom GLSL/HLSL shader support (this is a simple CPU-side pixel filter for Phase 1).
- CRT scanline or phosphor glow simulation (Phase 2).
- Modifying the actual NES PPU color palette generation (this operates on the final RGBA output).
