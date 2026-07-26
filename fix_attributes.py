with open("crates/nes-core/src/experimental/spatial_bot.rs", "r") as f:
    content = f.read()

content = content.replace("    #[test]\n    #[test]\n", "    #[test]\n")

with open("crates/nes-core/src/experimental/spatial_bot.rs", "w") as f:
    f.write(content)
