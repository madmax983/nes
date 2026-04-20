# 🔭 Vantage: Spec for Headless CI Mode

## 👤 User Story
"As a CI/CD Pipeline Engineer or Core Emulator Developer, I want to run the emulator in a 'headless' mode without a UI or audio/video drivers, so that I can automatically verify TAS replays and integration tests on server environments where display servers (like X11 or Wayland) and audio hardware are not available."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our `nes-desktop` emulator requires full GPU (wgpu/vulkan/metal) and audio (alsa/coreaudio) contexts to run. This makes it impossible to run automated functional tests or long-running AI training/evaluation tasks (`nes-ai`) in isolated, inexpensive Linux CI environments (like GitHub Actions runners) without relying on heavy and fragile virtual framebuffers (Xvfb). By creating a true headless mode, we unblock robust regression testing, speed up test execution by bypassing the renderer entirely, and reduce our CI infrastructure costs.

## 📊 Success Metrics
- **Portability:** The emulator can successfully launch and run a TAS script using `nes-desktop --headless` on a barebones Ubuntu server without crashing due to missing Wayland/X11 headers or audio drivers.
- **Performance:** Headless mode runs at "unlocked" speed, constrained only by CPU, completing a 10-minute TAS replay significantly faster than real-time.
- **Reliability:** CI pass rates for end-to-end replay tests reach 100% without intermittent failures related to virtual graphics drivers.

## 🕵️ Gap Analysis
- **Market View:** Mature emulators (like RetroArch or FCEUX) provide dedicated headless or "dummy" video/audio drivers specifically for server-side recording, testing, and AI training.
- **Our Gap:** While `nes-core` itself is pure logic and disconnected from I/O, our primary entry point for full-system integration tests and user macros (`nes-desktop`) hard-requires `winit` window creation and `wgpu` surface configuration before it starts the core execution loop.

## ✅ Acceptance Criteria
- Must introduce a `--headless` CLI flag to `nes-desktop`.
- Must suppress all UI window creation, avoiding `winit` event loop requirements.
- Must substitute the standard `wgpu` renderer and `symphonia`/`cpal` audio backend with "null" or "dummy" sinks that safely discard data.
- Must allow running a `.macro.txt` or `.tas.json` replay file to completion and exiting with a `0` status code upon success.
- Must still output critical error logs or panics to `stderr` if the emulation or replay fails.

## 🚫 Out of Scope
- Framebuffer extraction (saving screenshots from headless mode) - this is Phase 2.
- Headless netplay testing across multiple instances.
- Modifying the core `nes-core` execution loop; the changes should be isolated to the `nes-desktop` runner.
