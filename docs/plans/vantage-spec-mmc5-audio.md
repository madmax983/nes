# 🔭 Vantage: Spec for MMC5 Audio Support

## 👤 User Story
"As a Retro Gamer, I want the emulator to support the MMC5's audio expansion channels, so that games utilizing the expansion chip (like Akumajou Densetsu) sound full, accurate, and faithful to the original hardware experience."

## ❓ So What? (Business Problem)
What business problem does this solve?
While we have implemented the MMC5 mapper to handle PRG/CHR banking and standard logic, we currently stub the `$5000..=$5015` audio registers. This means popular MMC5 Famicom titles will have noticeably missing audio channels. Audio fidelity is a critical metric for emulator credibility. Implementing this fills a glaring gap in our MMC5 support, ensuring users don't switch to competitors for their favorite titles.

## 📊 Success Metrics
- **Performance:** Synthesizing the extra channels introduces zero audible stutter and maintains a steady 60fps.
- **Utility:** Games requiring MMC5 audio play with full, authentic instrumentation.

## 🕵️ Gap Analysis
- **Market View:** Other major emulators (Mesen, FCEUX) fully support MMC5 audio, setting a baseline expectation for any serious emulator.
- **Our Gap:** We have accurate MMC5 logic, but the 5B audio registers (`$5000..=$5015`) are stubbed and produce no sound.

## ✅ Acceptance Criteria
- Must synthesize the audio channels using the values written to `$5000..=$5015`.
- Audio must be mixed accurately with the core APU output.
- Performance must not degrade below 60fps when the extra audio channels are active.
- Save state serialization must capture the active phase and state of the audio channels.

## 🚫 Out of Scope
- Direct UI controls for isolating individual expansion audio channels (Phase 2).
