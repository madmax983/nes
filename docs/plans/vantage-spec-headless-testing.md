# 🔭 Vantage: Spec for Headless Testing

## 👤 User Story
"As an Emulator Developer, I want a headless execution mode that skips all UI rendering, so that I can run automated integration tests, fuzzing, and ROM verification pipelines quickly on CI servers without requiring a graphical environment."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Our CI pipeline currently requires a full desktop environment (X11/Wayland/Windows Desktop) or a complex `xvfb` setup to run the emulator because it is tightly coupled to the `winit` event loop and `pixels` framebuffer. This makes testing slow, fragile, and difficult to deploy in constrained CI environments. By implementing a headless execution mode, we decouple the core emulation logic from the presentation layer, enabling massively parallel, high-speed automated testing. This increases our iteration speed and software quality by allowing us to run comprehensive test suites on every commit reliably.

## 📊 Success Metrics
- **Performance:** Headless execution runs at least 10x faster than real-time when no frame rate limit is applied.
- **Utility:** Developers can run a full pass of the Blargg test suite on a headless Linux CI runner without any graphics dependencies.
- **Adoption:** 100% of automated CI test runs utilize the headless mode.

## 🕵️ Gap Analysis
- **Market View:** Modern emulator architectures (like libretro cores) strictly separate emulation from rendering, allowing easy testing.
- **Our Gap:** `nes-desktop` currently crashes if a window cannot be created. We have `nes-core`, but we lack a standalone driver binary designed specifically for fast-forward, I/O-less execution.

## ✅ Acceptance Criteria
- Must provide a `--headless` command-line flag to the emulator binary.
- Must bypass initialization of `winit`, `pixels`, and all OS-level windowing systems when the flag is present.
- Must execute the `nes-core` simulation loop continuously, capturing state and emitting logs or output as requested.
- Must support passing an input file (e.g., a TAS movie or macro script) to drive the emulator state without physical input.
- Must support exiting automatically based on a condition (e.g., a specific memory value being reached or an instruction limit).

## 🚫 Out of Scope
- Headless netplay testing (Phase 2).
- Video frame extraction/recording in headless mode (we are testing logic, not rendering).
