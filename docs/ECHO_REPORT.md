# 🗣️ Echo: AI Training getting started example fails

**Description:**

* 🤦 **The Confusion:** "I tried to follow the \`nes-ai\` getting started guide in \`README.md\`. I copied the config file: \`cp config/ai/profiles/smb-control.example.toml config/ai/profiles/smb-control.toml\`. Then I ran the training command \`cargo run -p nes-ai --bin train_smb_control -- ./config/ai/profiles/smb-control.toml ...\`. But it panicked because it couldn't find the ROM!"

* 🕵️ **The Reality:** "I checked the configuration file \`smb-control.example.toml\` and it has a hardcoded \`rom_path\` of \`./roms/Super Mario Bros.nes\`. But the repository doesn't include that ROM, the bundled ROM is in \`./roms/homebrew/homebrew.nes\`. The previous step in the README even uses the homebrew ROM!"

* 💡 **The Fix:** "Update \`config/ai/profiles/smb-control.example.toml\` so the \`rom_path\` points to a valid ROM that comes with the repo, like \`./roms/homebrew/homebrew.nes\`."
