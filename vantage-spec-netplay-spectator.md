# 🔭 Vantage: Spec for Netplay Spectator Mode

## 👤 User Story
"As an Esports Commentator or Tournament Organizer, I want to connect to a live netplay session as a spectator, so that I can broadcast the match to an audience without affecting the players' latency or rollback state."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our rollback netplay engine is strictly peer-to-peer (or via relay) limited to active participants. Broadcasting matches requires capturing a player's screen, which exposes their local rollbacks and UI, or relying on delayed VODs. By adding a spectator mode, we enable real-time, high-quality tournament broadcasting and community engagement. This positions our emulator as a viable platform for competitive retro gaming events, expanding our audience beyond just developers and solo players.

## 📊 Success Metrics
- **Performance:** Adding up to 10 spectators to a room does not increase the `net_rtt_ms` or `net_rollbacks` for the active players.
- **Utility:** Spectators receive a synchronized stream of inputs and can view the match with minimal perceivable delay.
- **Adoption:** 20% of netplay sessions hosted via the public relay involve at least one spectator connection.

## 🕵️ Gap Analysis
- **Market View:** Competitive platforms like Fightcade offer robust spectator features, allowing hundreds of users to watch live matches.
- **Our Gap:** Our `nes-relay` and `nes-netplay` crates currently only support routing for active participants. The relay does not broadcast input streams to non-participating clients, and the desktop client lacks a "Spectator" mode.

## ✅ Acceptance Criteria
- Must extend the `nes-relay` protocol to support a `SpectateRoom` command.
- Must allow `nes-desktop` to launch in a `--netplay-spectator` mode, connecting to a room without claiming a player slot.
- Must ensure the relay broadcasts confirmed inputs from active players to all connected spectators.
- Must prevent spectator clients from sending input data to the relay (read-only connection).
- Must ensure player clients do not wait for spectator acknowledgments to advance their simulation frames.

## 🚫 Out of Scope
- Spectator chat or social features (Phase 2).
- Rewinding or pausing the live spectator feed (Phase 2).
