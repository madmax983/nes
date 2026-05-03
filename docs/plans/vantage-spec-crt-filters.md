# 🔭 Vantage: Spec for CRT Filter Support

## 👤 User Story
"As a Nostalgic Player, I want to apply CRT scanline and phosphor glow video filters, so that my games look authentic and similar to how they appeared on my childhood television."

## 💼 Business Problem (So What?)
**What business problem does this solve?**
Currently, our emulator outputs perfectly crisp, raw pixels using nearest-neighbor scaling. While technically accurate, this looks harsh and overly sharp on modern high-resolution displays (1440p/4K). Emulation is as much about the "feel" as the logic. By introducing lightweight CRT shaders, we cater to the core retro-gaming enthusiast demographic, significantly improving the subjective visual quality and making the emulator feel like a premium product.

## 📈 Success Metrics
- **Performance:** Activating the default CRT shader adds less than 2ms of render overhead per frame, ensuring a locked 60fps on integrated graphics.
- **Adoption:** 25% of desktop users enable a video filter within their first week of use.

## 🕵️ The Reality:
- **Market View:** Top-tier emulators (e.g., RetroArch via Slang shaders, Mesen) provide extensive, highly customizable shader pipelines to mimic various display technologies.
- **Our Gap:** We currently have a highly performant `wgpu` rendering pipeline, but it lacks any post-processing capabilities. We only offer raw pixel scaling.

## ✅ Acceptance Criteria
- Must introduce a new "Video Filters" toggle in the desktop UI overlay menu.
- Must implement at least one default CRT shader utilizing our existing `wgpu` rendering pipeline (e.g., simple scanlines with slight screen curvature and bloom).
- Must apply the filter efficiently as a post-processing pass over the final composed PPU output.
- Must ensure the filter works correctly in both windowed and fullscreen modes without aspect ratio distortion.
- Must provide an option to disable the filter and return to raw nearest-neighbor output.

## 🚫 Out of Scope
- Full NTSC composite signal degradation/artifacting simulation (Phase 2).
- Support for importing custom, user-provided shader files (e.g., `.slang` support).
- Filter support for the `nes-web` or `nes-tui` targets (desktop only for Phase 1).
