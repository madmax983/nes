# 🔭 Vantage: Spec for Netplay Spectator Mode

## 👤 User Story:
As a tournament organizer or streamer, I want to join active netplay sessions as a passive observer, so that I can broadcast live matches without interfering with player latency or inputs.

## 💼 Business Problem (So What?):
Currently, broadcasting netplay matches requires capturing a player's screen, which introduces complexity and potential performance overhead on the player's machine. A spectator mode increases the project's utility for the competitive community by enabling frictionless, high-quality tournament streams.

## 📈 Success Metrics:
- Success = Spectator clients can connect and receive state updates without increasing latency or rollback frames for active players.

## 🔍 Gap Analysis:
- Need to expand relay capabilities to support passive broadcast connections alongside active player connections.

## ✅ Acceptance Criteria:
- Spectator clients can connect to an active `nes-relay` room.
- Spectator clients receive initial state and subsequent inputs but cannot send inputs.
- Host and peer players are unaware of or unimpacted by the spectator connection.
- Clear UI indication on the spectator client that it is in "Spectator Mode".

## 🚫 Out of Scope:
- Spectator chat or voice comms.
- Rewind capabilities for spectators during live matches (Phase 2).
- Server-side recording of matches (Phase 2).
