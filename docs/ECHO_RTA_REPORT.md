# 🗣️ Echo: RTA Getting Started example is broken

🤦 **The Confusion:**
I followed the "RTA mode (speedrunner-focused)" instructions in the `README.md` and tried to run the strict auto-select command:
`cargo run -p nes-desktop --release -- --rta --rta-profiles-dir ./config/rta/profiles ./roms/homebrew/homebrew.nes`

The emulator opened but RTA mode wasn't enabled. And when I tried the manual profile override command:
`cargo run -p nes-desktop --release -- --rta --rta-profile smb-any --rta-profiles-dir ./config/rta/profiles ./roms/homebrew/homebrew.nes`

It failed because `smb-any` isn't a valid profile!

🕵️ **The Reality:**
The directory `./config/rta/profiles` only contains `smb-any.example.toml`. There is no actual `.toml` profile file in that directory. The README mentions `config/rta/profiles/smb-any.example.toml` in passing later, but never tells the user to create a profile.

💡 **The Fix:**
Update the README to include a step for copying the example RTA profile before running the commands: `cp ./config/rta/profiles/smb-any.example.toml ./config/rta/profiles/smb-any.toml`.
