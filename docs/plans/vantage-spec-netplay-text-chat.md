# 🔭 Vantage: Spec for Netplay Text Chat

## 👤 User Story
"As a Netplay User, I want an in-game text chat system during multiplayer sessions, so that I can communicate with my opponent or co-op partner without needing a separate voice call or alt-tabbing to Discord."

## ❓ The So What? (Business Problem)
**What business problem does this solve?**
While our emulator currently supports robust rollback netplay (`nes-netplay`) and a room relay server (`nes-relay`), the experience is completely silent. Players who join public rooms via the lobby have no way to communicate, coordinate restarts, or chat about the match. This lack of communication prevents community building and forces users to rely on external platforms. Adding an integrated text chat keeps users immersed in the emulator, significantly improving the social experience and retention for random matchmaking.

## 📊 Metric Definition
- **Success =** 30% of Netplay sessions lasting longer than 5 minutes utilize the text chat feature.
- **Performance =** Chat overlay rendering and message processing add < 1ms to the frame budget to prevent rollback starvation.

## 🕵️ Gap Analysis
- **Market View:** Competitors like Fightcade have built their entire platform around seamless social interaction (lobbies, chat, spectating).
- **Our Gap:** We currently have the network infrastructure to transmit data, but we only send input deltas. We are missing the application-layer payload for text and the UI overlay to render it.

## ✅ Acceptance Criteria
- Must provide an unobtrusive UI overlay to display a scrolling chat history of at least the last 50 messages.
- Must provide a hotkey to focus the chat input box. When focused, game inputs must be captured by the chat box and not sent to the emulator core.
- Must transmit chat messages securely and reliably over the existing Netplay connection.
- Must display system messages in the chat feed (e.g., "Player 2 connected", "Ping: 45ms", "Rollback: 3 frames").
- Must display an indicator (e.g., a small notification pip) when a new message is received while the chat overlay is hidden.

## 🚫 Out of Scope
- Voice chat (Requires significant audio engineering and bandwidth management).
- Global/Lobby chat (This spec is scoped strictly to in-match, peer-to-peer/room communication).
- Complex chat formatting, emojis, or file sharing.
