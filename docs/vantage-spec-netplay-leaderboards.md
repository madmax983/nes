# 🔭 Vantage: Spec for Netplay Leaderboards

## 👤 User Story
"As a competitive Netplay Player, I want an integrated matchmaking rating (MMR) and leaderboard system, so that I can track my skill progression and compete against other players of similar skill levels."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our rollback netplay allows friends to play together via direct room connections. However, there is no public progression or matchmaking system to retain competitive players long-term. By introducing ranked play and leaderboards, we transform the emulator from a simple tool into a sticky, competitive platform, dramatically increasing player retention and daily active users (DAUs).

## 📊 Success Metrics
- **Engagement:** 20% increase in daily active netplay sessions.
- **Adoption:** 50% of players who play netplay participate in the ranked ladder.
- **Performance:** MMR calculations and leaderboard queries execute in < 50ms without blocking the core emulator loop.

## 🕵️ Gap Analysis
- **Market View:** Platforms like Fightcade provide robust matchmaking, ranked ladders, and global leaderboards that keep communities active for decades.
- **Our Gap:** We have solid peer-to-peer rollback netcode (`crates/nes-netplay`) and a room relay server (`crates/nes-relay`), but no persistence layer, user identity, or matchmaking logic.

## ✅ Acceptance Criteria
- Must introduce a "Ranked Mode" toggle in the desktop UI that queues players for matchmaking instead of joining a specific room.
- Must implement an Elo or Glicko-2 based rating system to calculate skill ratings after matches.
- Must provide a global leaderboard view within the emulator UI (or web counterpart) displaying top-ranked players.
- Must require user authentication (or persistent local identity tokens) to prevent smurfing and track match history securely.

## 🚫 Out of Scope
- Seasonal resets and rewards (Phase 2).
- In-client tournament organization (Phase 2).
