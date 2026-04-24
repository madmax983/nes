# 🔭 Vantage: Spec for OAM Zone Tracker API

## 👤 User Story
"As an AI Agent Developer or Auto-Tester, I want to define rectangular zones on the screen and receive events when sprites enter those zones, so that I can trigger logic (like dodging or jumping) without needing to poll every frame."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
While the `OamSpatialQuery` allows polling for sprites in an area, polling every frame is inefficient for external tools (like an AI agent running over MCP). The `ZoneTracker` provides an event-driven model: define the zones once, and the engine pushes notifications only when something crosses the boundary. Exposing this via the MCP turns our emulator into a high-performance event stream for AI integration, giving us a major competitive advantage in the AI/tooling space.

## 📊 Success Metrics
- **Performance:** Tracking up to 20 zones introduces < 1ms overhead per frame.
- **Accuracy:** Events fire exactly on the frame a sprite bounding box intersects a zone, and do not fire repeatedly while the sprite remains inside.
- **Adoption:** Used by `nes-ai` as the primary trigger mechanism for agent reactions.

## 🕵️ Gap Analysis
- **Market View:** Standard emulators do not have built-in spatial event trackers; this is usually handled by complex, game-specific Lua scripts.
- **Our Gap:** The `the internal zone tracking engine` is implemented but completely isolated from external interfaces like `nes-mcp`.

## ✅ Acceptance Criteria
- Must expose a new tool in the `nes-mcp` crate named `set_oam_zone`.
- Must accept parameters to define a zone: `id` (integer), `x`, `y`, `w`, and `h`.
- Must expose a new tool named `clear_oam_zones` to reset the tracking state.
- Must evaluate the defined zones every frame during the desktop run loop if any zones are active.
- Must expose a new tool named `poll_zone_events` (or push via MCP notifications if supported) that returns a JSON array of zone event objects triggered since the last poll.
- Must ensure that an event is only fired on *entry* (not continuously while inside).

## 🚫 Out of Scope
- Zone *exit* events (Phase 1 is entry-only).
- Pixel-perfect collision detection (bounding box based on the 8x8 OAM data is sufficient).
