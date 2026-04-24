# 🔭 Vantage: Spec for OAM Spatial Query API

## 👤 User Story
"As a Tooling Developer or AI Researcher, I want to execute spatial queries against the game's hardware sprites (OAM), so that I can automatically locate entities on screen without having to parse raw memory bytes or interpret the framebuffer pixels."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Writing AI agents or analysis tools for NES games usually requires deep, game-specific memory maps to find entity coordinates. The NES hardware naturally tracks all active sprites in the Object Attribute Memory (OAM). Our experimental `OamSpatialQuery` engine parses this data into structured bounding boxes. Exposing this through a structured API (like MCP) turns our emulator into a powerful, game-agnostic entity tracking engine, drastically lowering the barrier to entry for AI research and automated testing.

## 📊 Success Metrics
- **Performance:** Querying OAM data takes < 5ms and does not interrupt the active emulation frame.
- **Accuracy:** The returned sprite coordinates exactly match the PPU's internal representation for the current frame.
- **Adoption:** Used by downstream tooling (like `nes-ai`) as the primary method for tracking dynamic screen elements.

## 🕵️ Gap Analysis
- **Market View:** Some advanced tool-assisted speedrun (TAS) emulators allow viewing OAM, but rarely provide programmatic spatial querying (e.g., "find all sprites in this rectangle").
- **Our Gap:** The `the internal spatial query engine` is fully functional in the core but is not exposed to any external interface (like the MCP host).

## ✅ Acceptance Criteria
- Must expose a new tool in the `nes-mcp` (Model Context Protocol) crate named `get_ppu_oam_query`.
- Must accept parameters for a bounding box: `x` (0-255), `y` (0-255), `w` (0-255), and `h` (0-255).
- Must utilize the core's `the internal query logic` to find intersecting sprites.
- Must return a structured JSON array of matched sprites, including their OAM `index`, `x`, `y`, `tile_id`, and `attributes`.
- Must return an empty array if no sprites intersect the provided bounding box.

## 🚫 Out of Scope
- Reassembling meta-sprites (the NES hardware only knows about 8x8 or 8x16 hardware sprites; determining that a cluster of 6 hardware sprites makes up "Mario" requires game-specific logic, which is out of scope).
- Live on-screen bounding box visualizer overlay (Phase 2).
