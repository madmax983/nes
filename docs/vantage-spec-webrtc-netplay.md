# 🔭 Vantage: Spec for WebRTC Netplay

## 👤 User Story
"As a Browser Player, I want to connect directly to my friend's game without needing to download a desktop application, so that we can instantly play netplay sessions by just sharing a link."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our rollback netcode relies on a centralized relay server and is only available in the desktop application. This creates high friction for casual users who just want to play a quick game. By implementing peer-to-peer browser connections, we reduce our server costs, bypass restrictive firewalls, and provide a zero-install, instant-gratification multiplayer experience. This expands our addressable market to casual social gamers and drastically reduces user acquisition friction.

## 📊 Success Metrics
- **Adoption:** 30% of all netplay sessions are initiated via the browser client within 3 months of launch.
- **Latency:** Browser data channel connections achieve an average RTT within 15ms of raw desktop connections for the same geographical pairing.
- **Reliability:** 95% of connection attempts successfully establish a data channel.

## 🕵️ Gap Analysis
- **Market View:** Modern browser-based emulators and retro gaming platforms (like Jam.gg) utilize browser-to-browser protocols to offer seamless, link-based multiplayer without requiring downloads.
- **Our Gap:** We have the browser build target and the deterministic rollback engine, but they do not currently communicate. Our browser build is strictly single-player.

## ✅ Acceptance Criteria
- Must provide a "Host Netplay" option in the web UI that generates a unique, shareable join link.
- Must establish a peer-to-peer Data Channel between two browser clients to exchange state.
- Must route the existing rollback inputs through the peer-to-peer Data Channel instead of standard sockets.
- Must handle temporary connection drops gracefully, utilizing the existing rollback mechanisms to resync state upon reconnection.
- Must provide clear UI feedback during the connection phase (e.g., "Waiting for Peer", "Connected").

## 🚫 Out of Scope
- Voice or text chat over the connection (Phase 2).
- Support for more than two concurrent players (spectators or 3+ player games).
- Direct peer-to-peer connections for the desktop client (this spec focuses solely on the browser implementation).
