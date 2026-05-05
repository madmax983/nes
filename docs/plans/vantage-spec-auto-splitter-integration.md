# 🔭 Vantage: Spec for Auto-Splitter Integration

## 👤 User Story
"As a Speedrunner, I want the emulator to automatically trigger splits in my timer software based on in-game memory values, so that I can focus purely on gameplay without worrying about manual split inaccuracies."

## 💼 Business Problem (So What?)
**What business problem does this solve?**
Our current RTA mode auto-selects profiles and generates run reports, but runners still rely on hotkeys (F9) for manual splits or use external tools to read emulator memory. This creates friction and limits adoption by top-tier runners who demand frame-perfect split tracking. By exposing a standard auto-splitter interface (e.g., LiveSplit Server protocol or a websocket API), we make our emulator a zero-configuration, tournament-ready platform for speedrunning, directly capturing the competitive community.

## 📈 Success Metrics
- **Accuracy:** Splits trigger exactly on the designated frame without delay.
- **Compatibility:** Integrates seamlessly with LiveSplit out of the box.
- **Adoption:** 80% of RTA runs use the auto-splitter feature within a month of release.

## 🕵️ The Reality:
- **Market View:** Specialized emulators and tools provide memory hooks for LiveSplit, but often require complex setup, specific emulator versions, or custom scripts.
- **Our Gap:** We track RTA stats internally and have deep access to memory state, but we don't broadcast these state changes to external timer applications in real-time.

## ✅ Acceptance Criteria
- Must expose a configurable interface (e.g., WebSocket) for external timer tools to connect to.
- Must allow RTA profiles to define specific memory conditions (e.g., address X transitions from A to B) that trigger a split event.
- Must emit a clear, standardized JSON payload for each split event over the interface.
- Must not introduce measurable latency to the core emulation loop.

## 🚫 Out of Scope
- Building our own standalone timer UI.
- Supporting legacy or obscure timer protocols (focus on the industry standard, LiveSplit).
