# DX Audit Plan for Echo

1. **Verify README Run Examples:**
   - Review all ````powershell` blocks in `README.md`.
   - Notice that they explicitly use `powershell` but the examples assume `pwsh` (PowerShell Core) which might not be installed on Linux/Mac environments by default.
   - The example scripts in `scripts/` are `.ps1` files and are executed with `powershell -NoProfile -ExecutionPolicy Bypass -File ...`.
   - On Linux/Mac environments `powershell` command might be absent or `pwsh` should be used instead. Or better yet, we should provide `.sh` scripts.
   - Furthermore, some paths use backslashes `.\roms\homebrew\homebrew.nes` in the README and some use forward slashes `./roms/homebrew/homebrew.nes`.
2. **Action - Create an issue/PR for Docs Fix:**
   - I will submit a PR with a "Docs Fix" request because the README examples contain un-runnable powershell scripts for linux users (and `pwsh` is missing from the environment), plus backslash/forward slash inconsistency.
   - I will format the PR title and description as required by the Echo persona guidelines.

```markdown
Title: 🗣️ Echo: Getting Started examples use Windows-specific powershell and paths

🤦 **The Confusion:** Tried to run the README examples for automation scripts and web demo. `powershell` and `pwsh` commands were not found on my Linux machine, so I couldn't run the `mcp_play_demo.ps1` or `run_homebrew.ps1` scripts. Also, some paths use Windows-style backslashes (`.\roms\homebrew\homebrew.nes`) while others use forward slashes.

🕵️ **The Reality:** Turns out the project relies heavily on PowerShell scripts (`.ps1`) for its automation and build runners, which assumes a Windows environment or a manual installation of PowerShell Core (`pwsh`) on Linux/macOS.

💡 **The Fix:** Add native shell scripts (`.sh`) for Linux/macOS users alongside the `.ps1` scripts, or add a big banner in the README explaining that PowerShell Core (`pwsh`) is required to run the scripts on non-Windows platforms. Also standardize path separators in the documentation to use forward slashes (`/`) which are compatible across most modern shells including PowerShell.
```
