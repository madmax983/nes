# 🔭 Vantage: Spec for Netplay Spectator Mode

## 👤 User Story
"As an Esports Organizer or Community Broadcaster, I want a zero-latency spectator mode for ongoing netplay sessions, so that I can broadcast high-quality, lag-free gameplay with a clean UI without impacting the competitors' rollback latency."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our `nes-netplay` engine supports excellent peer-to-peer rollback for two active players, coordinated via `nes-relay`. However, broadcasting a tournament or sharing gameplay requires one of the players to screen-share over Discord or Twitch, introducing visual compression, latency, and adding encoder overhead to the player's machine. By allowing a third instance to connect to the `nes-relay` as a passive, non-authoritative client that receives inputs directly from the relay server, we enable professional-grade, high-quality broadcasts. This dramatically increases the visibility and viability of our emulator for competitive play and community events.

## 📊 Success Metrics
- **Performance:** Connecting a spectator client adds zero additional network latency or CPU overhead to Player 1 or Player 2.
- **Utility:** A spectator client can reliably reconstruct the game state and render at 60fps purely from the relayed input stream.
- **Adoption:** 100% of community-run tournaments utilizing our netplay engine use the spectator mode for their official broadcast feed.

## 🕵️ Gap Analysis
- **Market View:** Premium modern fighting games (e.g., GGPO-based titles) and advanced netplay emulators (like Slippi for Smash Bros) feature robust broadcast/spectator modes that decouple the viewer from the players' local rendering.
- **Our Gap:** The current `nes-netplay` protocol only supports two authoritative peers exchanging inputs. `nes-relay` simply forwards packets between these two fixed peers. There is no concept of a read-only client or a mechanism for a late-joining client to sync the initial ROM state and catch up to the current frame.

## ✅ Acceptance Criteria
- Must introduce a `Spectator` role to the `nes-netplay` protocol and `nes-relay` server logic.
- Must allow a spectator client to connect to an active room via `nes-desktop --netplay --netplay-room <room> --netplay-spectator`.
- Must provide a mechanism for the spectator to synchronize the initial game state (e.g., receiving a compressed savestate from the relay or Player 1 upon connection).
- Must seamlessly apply relayed inputs from both players to keep the spectator's local `NesCore` instance in sync with the live game.
- Must ensure that packets sent to the spectator are strictly one-way; the spectator cannot introduce rollbacks or delays to the active players.

## 🚫 Out of Scope
- Multi-spectator broadcasting directly from the relay server (Phase 1 supports a single dedicated broadcaster client; massive scale requires a different architecture).
- In-game chat or interaction between players and the spectator.
- Re-broadcasting the PPU video feed (Spectator runs its own full emulation locally).
