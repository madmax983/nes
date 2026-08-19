# 🔭 Vantage: Spec for Game Genie Support

## 👤 User Story
"As a casual retro gamer, I want to input Game Genie codes, so that I can modify game behavior (e.g., infinite lives, invincibility) and complete difficult classic games without frustration."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Many users play classic NES games for nostalgia but lack the time or patience to overcome their notoriously high difficulty ("Nintendo Hard"). By failing to support cheat codes natively, we force these users to seek alternative emulators or give up on their sessions early. Native Game Genie support increases user retention, session length, and broadens our target audience beyond just hardcore purists and developers.

## 📊 Success Metrics
- **Adoption:** 20% of users loading commercial ROMs enable at least one cheat code during their session.
- **Retention:** Users utilizing cheat codes show a 30% increase in average session duration.
- **Reliability:** 0% crash rate when applying valid 6-letter or 8-letter Game Genie codes.

## 🕵️ Gap Analysis
- **Market View:** Almost all mainstream consumer NES emulators (e.g., FCEUX, Nestopia, Mesen) provide robust Game Genie and Pro Action Replay cheat support, often with built-in cheat databases.
- **Our Gap:** We currently focus heavily on strict determinism, accuracy, and TAS/developer tooling. We have no user-facing facility to patch ROM or RAM values dynamically on the fly, which is a baseline expectation for a modern emulator.

## ✅ Acceptance Criteria
- Must support standard 6-letter and 8-letter Game Genie codes.
- Must provide an interface (via `nes-desktop` or `nes-mcp`) to add, toggle, and remove cheat codes dynamically during gameplay.
- Must correctly intercept CPU memory reads to apply active cheat patches without permanently altering the loaded ROM data.
- Must cleanly isolate cheat state so that it can be disabled, or prominently flagged/warned against, during Netplay, strict TAS recording sessions, or leaderboards.

## 🚫 Out of Scope
- Creating a cheat search/memory scanning tool (Phase 2).
- Pro Action Replay (PAR) code support.
- Bundling a hardcoded database of game-specific cheat codes (users must supply their own codes initially).
