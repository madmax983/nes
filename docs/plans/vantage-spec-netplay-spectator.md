# 🔭 Vantage: Spec for Netplay Spectator Mode

## 👤 User Story
"As a Netplay User or Tournament Organizer, I want to watch an active netplay session without taking up a player slot, so that I can broadcast the match, learn from others, or enjoy the game socially."

## 💼 Business Problem (So What?)
**What business problem does this solve?**
Currently, our netplay implementation (`nes-netplay` and `nes-relay`) strictly expects a fixed 2-peer connection for rollback simulation. There is no way for a third party to view an ongoing match without disrupting the determinism or occupying a controller slot. By adding a dedicated Spectator Mode, we expand our netplay ecosystem from a private 1v1 experience to a community-driven, broadcast-friendly platform. This enables tournament streaming and casual social viewing, significantly driving user engagement and organic growth.

## 📈 Success Metrics
- **Performance:** Spectator clients connecting to a relay room do not negatively impact the `net_rtt_ms` or `net_rollbacks` of the active players.
- **Utility:** A spectator can connect mid-match and seamlessly catch up to the current frame state.
- **Adoption:** 20% of netplay sessions hosted on public relays have at least one spectator connected.

## 🕵️ Gap Analysis
- **Market View:** Modern fighting games (like GGPO-based titles) and major emulator platforms (like Fightcade) offer robust spectator support natively, allowing hundreds of users to watch a single lobby in real-time.
- **Our Gap:** Our current relay protocol only routes inputs between Player 1 and Player 2. We lack the protocol messages to declare a "spectator" role, broadcast inputs to non-playing peers, and synchronize the initial ROM/save-state for mid-match joiners.

## ✅ Acceptance Criteria
- Must update the relay server protocol to support a dedicated spectator connection type.
- Must ensure that the relay server broadcasts both P1 and P2 inputs to all connected spectators in real-time.
- Must ensure that spectator clients *never* send input data to the relay server (read-only mode).
- Must provide a mechanism for mid-match joiners to sync state (e.g., relaying the initial savestate or starting from a known keyframe).
- Must display "Spectating [Room Name]" clearly in the desktop/TUI client when connected as a spectator.

## 🚫 Out of Scope
- Spectator text chat or voice chat (Phase 2).
- Supporting more than 10 spectators per room on a single standard relay instance (Scale optimization is Phase 2).
- Rewinding or fast-forwarding the match from the spectator's view (live view only for Phase 1).
