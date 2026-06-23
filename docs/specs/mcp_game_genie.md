# 🔭 Vantage: Spec for MCP Game Genie Cheats

👤 **User Story:** "As an AI Agent or MCP User, I want to manage and apply Game Genie cheat codes via the Model Context Protocol, so that I can programmatically bypass difficulty hurdles or manipulate game state during automated runs."

💼 **So What? (Business Problem):**
While `nes-core` and `nes-desktop` robustly support Game Genie codes, `nes-mcp` does not expose them. This limits the AI's ability to autonomously perform tasks that require specific game states (like infinite lives or starting on a specific level) without complex macro scripting. By exposing cheat management, we increase the utility of the MCP interface for advanced automation and training scenarios.

📈 **Success Metrics:**
- Agents can successfully add, toggle, and remove cheat codes via standard MCP tools.
- Cheat states correctly map to the underlying emulator core without panicking.

🔍 **Gap Analysis:**
- The underlying logic exists in `nes_core::CheatCodes` and `nes_desktop::session_cheats`.
- The gap is entirely in the `nes-mcp` tool catalog (`crates/nes-mcp/src/tools.rs`) and dispatch layer (`crates/nes-mcp/src/dispatch.rs`).

✅ **Acceptance Criteria:**
- New MCP tools must be defined: `add_cheat`, `remove_cheat`, `clear_cheats`, and `list_cheats`.
- Must return descriptive errors for invalid Game Genie codes.
- Changes must be isolated to the MCP surface; core behavior remains unchanged.

🚫 **Out of Scope:**
- Implementing the "Cheat Finder" memory scanning algorithm over MCP (Phase 2).
- Persisting cheat codes across application restarts.
