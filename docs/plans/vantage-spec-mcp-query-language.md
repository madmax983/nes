# 🔭 Vantage: Spec for MCP Query Language

## 👤 User Story
"As an AI Agent or Automation Tooling Developer, I want a structured query language to inspect the emulator's memory and state, so that I can programmatically analyze running games without writing hardcoded memory offsets for every title."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our Model Context Protocol (MCP) interface provides basic, rigid endpoints for state inspection. This limits the ability of external AI agents (like `nes-ai`) and automation scripts to dynamically explore and understand the game state. By introducing a flexible MCP Query Language, we transform the emulator from a passive execution engine into an interactive, interrogatable database of game state, drastically reducing the friction for AI training and complex tool development.

## 📊 Success Metrics
- **Performance:** Query execution evaluates in under 5ms per frame.
- **Utility:** A single query can successfully locate the player's X coordinate dynamically by searching for known value changes across frames.
- **Adoption:** 100% of internal `nes-ai` scripts migrate from hardcoded memory offsets to the new query language.

## 🕵️ Gap Analysis
- **Market View:** Other tool-assisted emulators (e.g., FCEUX) rely on Lua scripting for dynamic memory interaction, which requires a heavy embedded runtime.
- **Our Gap:** We currently expose basic `read_byte` commands over MCP, but lack a way to perform aggregate searches, conditional watches, or structured data extraction over the network.

## ✅ Acceptance Criteria
- Must define a simple, text-based query syntax (e.g., similar to SQL or jq) that can be sent over the MCP connection.
- Must support memory range scanning (e.g., `SELECT address FROM wram WHERE value == 0x05`).
- Must support watching specific addresses for changes across frames.
- Must return results in a structured, machine-readable format (JSON).
- Must execute queries deterministically and not mutate the emulator state.

## 🚫 Out of Scope
- A graphical user interface for building queries (Phase 2).
- Queries that mutate memory state (read-only for Phase 1).
- Complex joins or cross-frame historical queries beyond single-frame state.
