# 🔭 Vantage: Spec for Hitbox Visualizer

## 👤 User Story
"As a TAS Creator or AI Researcher, I want a real-time visual overlay showing sprite bounding boxes (hitboxes), so that I can precisely analyze collision geometry, positioning, and object interactions without manually reading OAM memory dumps."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
While our emulator provides exact execution and programmatic memory access, visual debugging for physics and collisions is currently impossible without external tools. TAS creators and AI researchers rely heavily on spatial reasoning (e.g., determining exact pixel distances between a player and an enemy). We already have the experimental `OamSpatialQuery` engine (`crates/nes-core/src/experimental/oam_spatial_query.rs`) tracking OAM sprites. Exposing this data visually transforms the emulator into a first-class analytical workstation, reducing dependency on external memory viewers and improving workflow speed for power users.

## 📊 Success Metrics
- **Performance:** Rendering the hitbox overlay introduces less than 1ms overhead per frame on average.
- **Utility:** Researchers can visually confirm sprite boundaries that perfectly match the programmatic `OamSpatialQuery` output.
- **Adoption:** 20% of users utilizing TAS recording features or the `nes-ai` stack enable the hitbox visualizer during active sessions.

## 🕵️ Gap Analysis
- **Market View:** TAS-focused emulators (like FCEUX or BizHawk) offer robust Lua scripting or native overlays to draw geometric shapes representing hitboxes.
- **Our Gap:** We currently only display the composite video output. Even though our `OamSpatialQuery` engine decodes sprite positions programmatically for headless queries, we lack any visual representation in `nes-desktop` or `nes-tui` to assist human reasoning.

## ✅ Acceptance Criteria
- Must provide a toggleable UI option (via `nes-desktop` or a hotkey) to enable the "Hitbox Overlay".
- When enabled, must draw a rectangular border (e.g., 8x8 pixels) over every valid active sprite derived from the `OamSpatialQuery` engine.
- Must ensure the overlay geometry correctly maps to the scaled output frame (accounting for aspect ratio and window resizing).
- Must disable the overlay automatically and silently if the `nova` feature is not active (since `OamSpatialQuery` is gated behind `nova`).

## 🚫 Out of Scope
- Background (Nametable) collision visualizers (Phase 2).
- Distinguishing between "Player", "Enemy", or "Projectile" hitboxes automatically (purely rendering OAM bounding boxes).
- Sub-pixel precise hitboxes derived from RAM variables (only raw OAM positions are visualized).
