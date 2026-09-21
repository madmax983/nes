# 🗣️ Echo: AI Control Training example crashes with missing ROM

🤦 **The Confusion:**
I followed the "AI Control Training" steps in the `README.md`. I successfully ran step 1 (`prepare_smb_control`) which used the bundled homebrew ROM (`./roms/homebrew/homebrew.nes`). I then copied the example config and ran step 2 (`train_smb_control`). It immediately crashed with:
`Training failed: failed to read ROM './roms/Super Mario Bros.nes': No such file or directory (os error 2)`

🕵️ **The Reality:**
The README states we can use "the bundled homebrew ROM for demonstration", but the `smb-control.example.toml` configuration file hardcodes `rom_path = "./roms/Super Mario Bros.nes"`. The training script tries to load a copyrighted ROM I don't have, instead of the homebrew ROM I just generated the snapshot for.

💡 **The Fix:**
Update `config/ai/profiles/smb-control.example.toml` to point to `./roms/homebrew/homebrew.nes` by default, or add a warning/instruction in the README telling the user they must manually edit `rom_path` in their `smb-control.toml` before running step 2.

---

# 🗣️ Echo: Strict RTA mode example fails to match ROM hash

🤦 **The Confusion:**
I ran the command from the "Strict RTA mode (auto-select profile by ROM hash)" section in the README:
`cargo run -p nes-desktop --release -- --rta --rta-profiles-dir ./config/rta/profiles ./roms/homebrew/homebrew.nes`
It failed to start and threw this error:
`Failed to enter RTA mode for ROM hash f2bc46167653d83303252f689b6fd4f613ff7fb47f8ca7526a9c24bffc74cd3d: No RTA profile matched ROM hash f2bc46167653d83303252f689b6fd4f613ff7fb47f8ca7526a9c24bffc74cd3d. Known profiles: [smb-any]. Provide --rta-profile <id> to override.`

🕵️ **The Reality:**
The README command passes the homebrew ROM (`homebrew.nes`), but the only available RTA profile (`smb-any.example.toml`) is mapped to the Super Mario Bros ROM hash. Strict mode auto-selection fails because there is no matching profile for the homebrew ROM.

💡 **The Fix:**
Either change the README command to use the manual profile override (like the second example does), or provide an `rta/profiles/homebrew.example.toml` profile so the strict auto-select command actually works out of the box.
