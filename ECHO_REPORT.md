# 🗣️ Echo: Strict RTA mode example is confusing

* 🤦 **The Confusion:** Tried to run the "Strict RTA mode" example `cargo run -p nes-desktop --release -- --rta --rta-profiles-dir ./config/rta/profiles ./roms/homebrew/homebrew.nes`. It crashed with `Failed to enter RTA mode for ROM hash... No RTA profile matched ROM hash`.
* 🕵️ **The Reality:** The workspace bundles a `homebrew.nes` ROM, but strict RTA mode requires a ROM whose hash exactly matches a known profile (like `smb-any`). The homebrew ROM doesn't match the Super Mario Bros profile hash, causing it to fail instantly.
* 💡 **The Fix:** Change the README example to use a hypothetical path like `<path-to-super-mario-bros.nes>`, or add a note explaining that users must create a profile matching their ROM's hash first before strict auto-select will work.
