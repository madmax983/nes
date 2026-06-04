# 🔭 Vantage: Spec for ROM Browser GUI

## 👤 User Story
"As a Player or Developer, I want a graphical ROM Browser within the emulator UI, so that I can easily discover, manage, and launch my `.nes` files without typing long command-line paths."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
While CLI arguments are powerful for scripting and advanced users, the vast majority of standard users expect a "plug-and-play" experience. Forcing users to launch a desktop emulator exclusively via the terminal or by writing configuration files creates a massive friction point, leading to early abandonment. By introducing a visual ROM browser, we immediately lower the barrier to entry, increasing active sessions, reducing "how to launch" support requests, and capturing casual players in addition to our core homebrew/developer audience.

## 📊 Success Metrics
- **Performance:** Browsing a directory of 500+ ROMs incurs no noticeable scroll lag or UI blocking.
- **Utility:** A user can locate and launch a specific ROM in under 3 clicks from startup.
- **Adoption:** 80% of `nes-desktop` launches without CLI arguments successfully result in a loaded ROM via the browser.

## 🕵️ Gap Analysis
- **Market View:** Virtually every mainstream emulator (Nestopia, FCEUX, Mesen) features a "File -> Open" dialog or a dedicated ROM library view.
- **Our Gap:** We currently require `cargo run -- <path_to_rom>` or dragging files onto the executable (which may not always work reliably across OS). Launching the emulator without a ROM just shows a blank/waiting state. We lack a native, discoverable way to find playable content.

## ✅ Acceptance Criteria
- Must display by default when `nes-desktop` is launched without a ROM argument.
- Must display a grid or list view of `.nes` files located in a configured or default `roms/` directory.
- Must extract and display the internal ROM header information (e.g., Mapper, PRG size, CHR size) upon selection.
- Must allow launching the selected ROM with a single double-click or "Play" button.
- Must support basic filtering or search by filename.
- Must remember the last browsed directory across sessions.

## 🚫 Out of Scope
- Scraping box art or metadata from external online databases (Phase 2).
- Automatic sorting of ROMs by genre or release year.
- Integrating with cloud saves within the browser UI.
