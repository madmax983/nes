# 🔭 Vantage: Spec for Game Metadata Scraper

## 👤 User Story
"As a Game Collector, I want the emulator to automatically fetch and display box art and release metadata for my ROMs, so that my game library feels polished and visually appealing."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
While we have a highly robust and accurate emulator core, the user experience outside of direct gameplay remains largely utilitarian (relying on command-line paths, basic file dialogs, and raw ROM filenames). By providing a rich, visual game library enriched with metadata, we elevate our software from a mere "execution tool" to a "premium gaming library application". This reduces friction in managing large collections, provides immediate visual recognition of games, and matches the polished library experience users expect from modern platforms like Steam or RetroArch, ultimately increasing user retention and satisfaction.

## 📊 Success Metrics
- **Match Rate:** 80% of standard, unmodified commercial ROMs successfully match and download accurate box art and metadata without manual intervention.
- **Performance:** Scanning and scraping a library of 100 ROMs takes less than 15 seconds, and browsing the cached library UI maintains 60fps.
- **Adoption:** 50% of Desktop users transition from single-ROM command-line launches to using the internal visual library view within a month of release.

## 🕵️ Gap Analysis
- **Market View:** Frontends like RetroArch, emulation stations like OpenEmu, and dedicated launchers provide rich library views with scraped metadata, box art, and contextual information.
- **Our Gap:** We currently only deal with raw file paths and file hashes. Users have no visual representation or contextual information about their collection within the emulator itself.

## ✅ Acceptance Criteria
- Must calculate the MD5/SHA1 hash of added ROM files to query a public metadata API (e.g., IGDB, ScreenScraper, or TheGamesDB).
- Must download and cache box art images locally in a dedicated `config/library/boxart` directory to prevent repeated API calls.
- Must extract and store basic metadata (Title, Release Year, Publisher, Genre) in a local library database or JSON index.
- Must display the box art alongside the title and release year in a "Library View" UI for the Desktop client.
- Must gracefully fallback to displaying the raw filename and a generic cartridge icon if a ROM is unrecognized or if the user is offline.
- Must provide a manual "Rescrape" or "Refresh" option for individual titles.

## 🚫 Out of Scope
- Scraping game manuals, video snaps, or audio previews (Phase 2).
- Manual metadata editing UI or custom image uploading (Phase 2).
- WebAssembly (nes-web) library integration (Desktop and TUI only for Phase 1).
