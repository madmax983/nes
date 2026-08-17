# 🔭 Vantage: Spec for Interactive Controller Remapping UI

## 👤 User Story
As a Desktop Player, I want to visually remap my controller and keyboard inputs within the emulator UI, so that I can customize my controls without needing to manually edit configuration files.

## ❓ So What?
**What business problem does this solve?**
Currently, users must manually edit `nes.toml` to change their input bindings. This creates friction for non-technical users. By providing an interactive remapping UI, we eliminate this barrier to entry, improving user retention and making the emulator feel like a consumer-ready product.

## 📈 Success Metrics
- Success = 0 manual edits to `nes.toml` required to change controller mappings for standard USB gamepads.
- Success = Time to remap a full controller is under 30 seconds.

## 🕵️ Gap Analysis
- Market View: Modern consumer emulators provide an in-app visual controller configuration screen.
- Our Gap: We currently rely on manual text-file editing for input configuration.

## ✅ Acceptance Criteria
- Must provide a dedicated UI overlay to remap Player 1 and Player 2 inputs.
- Must support both Keyboard and Gamepad inputs.
- Must listen for the next input (keypress or button press) when assigning a button.
- Must save the updated mappings back to the active configuration file (`nes.toml`).
- Must apply the new mappings instantly without requiring an emulator restart.

## 🚫 Out of Scope
- Support for analog stick deadzone configuration.
- Support for turbo/macro button combinations.
