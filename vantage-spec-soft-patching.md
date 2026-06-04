# 🔭 Vantage: Spec for Runtime Soft-Patching (IPS/BPS)

## 👤 User Story
"As a Retro Gamer and ROM Hacker, I want the emulator to automatically apply IPS or BPS patches to my ROM at runtime, so that I can play fan translations or romhacks without modifying my original clean ROM files."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
ROM hacks and fan translations represent a massive portion of the NES emulation ecosystem. Currently, users must use external third-party tools to permanently patch their ROMs, cluttering their library, risking the destruction of verified clean dumps, and adding friction to the gaming experience. By supporting in-memory soft-patching, we remove this friction, protect the integrity of the user's ROM library, and align our emulator with modern user-friendly standards found in competing products.

## 📊 Success Metrics
- **Performance:** Applying a patch at startup adds less than 100ms to the ROM load time.
- **Utility:** Zero permanent modifications to the original `.nes` file on disk.
- **Adoption:** 20% of ROMs loaded have a corresponding patch applied seamlessly at runtime.

## 🕵️ Gap Analysis
- **Market View:** Leading emulators (like RetroArch, Mesen, Snes9x) natively support soft-patching by automatically looking for `.ips` or `.bps` files that match the ROM's filename.
- **Our Gap:** We currently only load raw `.nes` files. Users wishing to play a romhack must hard-patch their files externally before loading them into `nes-desktop` or `nes-web`.

## ✅ Acceptance Criteria
- Must automatically detect and apply `.ips` or `.bps` files with the same base filename in the same directory as the loaded ROM.
- Must apply the patch entirely in-memory (soft-patching) without altering the original ROM file on disk.
- Must log the successful application of the patch to the console/UI.
- Must handle malformed or incompatible patches gracefully (e.g., fallback to the unpatched ROM with a clear warning).

## 🚫 Out of Scope
- In-UI patch creation tools (e.g., diffing two ROMs to generate a patch).
- Support for complex multi-patching or managing explicit load orders.
- Hard-patching features (saving the modified ROM back to disk).
