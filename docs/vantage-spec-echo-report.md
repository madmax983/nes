# 🔭 Vantage: Spec for Copying Default Config from README

## 👤 User Story
"As a New User, I want clear instructions to copy the example configuration, so that I don't run into a file not found error when I launch the emulator for the first time."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, new users encounter a "failed to read config" crash when executing the commands listed in the README, because the `nes.toml` file does not exist by default. The user must figure out that they need to copy `nes.example.toml` on their own, adding onboarding friction. This fix addresses the direct user complaint filed in `ECHO_REPORT.md` by explicitly adding the necessary setup step to the README.

## 📊 Success Metrics
- **Onboarding Success:** Users following the README instructions successfully launch the emulator without errors.
- **Zero Confusion:** No new issues related to `nes.toml` not found when following quickstart guide.

## 🕵️ Gap Analysis
- **Market View:** Quickstart guides typically include all necessary setup steps, including initializing configuration files from templates.
- **Our Gap:** The README mentions `nes.toml` but omits the command to create it from the provided example, leading to immediate failure of the very first command users are instructed to run.

## ✅ Acceptance Criteria
- A command to copy the example configuration must be added to the README.
- The command must appear immediately before the first `cargo run` commands.
- The command must clearly instruct the user to `cp nes.example.toml nes.toml`.

## 🚫 Out of Scope
- Automatically generating the configuration in code (this is addressed in a separate spec).
