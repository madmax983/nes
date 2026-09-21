# 🔭 Vantage: Spec for Homebrew Debugger

## 👤 User Story
"As a Homebrew Developer, I want an interactive frame-by-frame debugger and memory viewer overlay, so that I can step through my game's CPU instructions, inspect memory values, and track down logic bugs without leaving the emulator."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our emulator provides exceptional accuracy, a highly deterministic core, and experimental tooling like the `CpuHotspotProfiler`. However, we lack an integrated visual debugging environment. Developers must rely on external tools or raw command-line traces to understand why their game logic failed. By integrating a visual debugger (CPU step, memory hex editor, register inspector), we position our emulator as the premier, all-in-one development workstation for modern NES homebrew, capturing the active developer community.

## 📊 Success Metrics
- **Performance:** Activating the debugger overlay adds zero performance overhead when not paused/stepping.
- **Utility:** Developers can set a breakpoint, hit it, step forward exactly one instruction, and view the updated accumulator value in a single session.
- **Adoption:** 50% of users loading the custom `homebrew.nes` utilize the debugger overlay during their session.

## 🕵️ Gap Analysis
- **Market View:** Specialized development emulators (like FCEUX or Mesen) have robust debugging suites including piano rolls, PPU viewers, and memory hex editors.
- **Our Gap:** We have the technical foundation (e.g., `CpuHotspotProfiler`, core memory access, accurate CPU stepping), but we do not expose this through any user-facing UI or provide interactive stepping controls. F5/F8 savestates or Time Machine rewinds are reactive; developers need proactive, precise inspection.

## ✅ Acceptance Criteria
- Must provide an interactive UI overlay (via `nes-desktop` or `nes-tui`) to toggle "Debug Mode".
- Must allow pausing emulation and stepping exactly one CPU instruction or one PPU frame at a time.
- Must display current CPU registers (A, X, Y, PC, SP, Status Flags) in real-time while stepping.
- Must display a live scrolling window of disassembled CPU instructions around the current PC.
- Must provide a memory viewer (hex editor layout) with the ability to jump to specific addresses (e.g., Zero Page, stack).
- Must prevent conflicting interactions (e.g., disable Time Machine rewinds or netplay rollback while debugging).

## 🚫 Out of Scope
- Real-time PPU pattern table / nametable viewers (Phase 2).
- Advanced conditional breakpoints (Phase 2).
- Modifying memory directly via the hex editor (read-only inspection for Phase 1).
- In-browser (WASM) debugger UI.
