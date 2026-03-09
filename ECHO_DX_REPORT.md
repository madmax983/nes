# 🗣️ Echo: Getting Started example expects deep iNES knowledge instead of loading a file

**Description:**

* 🤦 **The Confusion:** Tried to use the `NesCore` example in `crates/nes-core/src/api.rs` to run my game. Instead of showing me how to load a `.nes` file, the example constructs a `dummy_rom` array using magic bytes (`0x4E, 0x45, 0x53, 0x1A`) and manual byte padding for PRG/CHR banks! As a new user, I have no idea what these bytes mean. I just want to load my ROM.
* 🕵️ **The Reality:** The emulator requires a raw byte slice for `load_ines_rom`, but the example completely obscures how a real user would supply their file. It expects me to know the internal structure of an iNES header just to pass the first example.
* 💡 **The Fix:** Replace the `dummy_rom` array construction in the `NesCore` `/// ## Examples` block with a simple file read, like `let rom_bytes = std::fs::read("my_game.nes").unwrap();`. Use the `no_run` attribute on the doc-test so it compiles without needing a real file on disk.
