with open("crates/nes-core/src/experimental/spatial_bot.rs", "r") as f:
    content = f.read()

# Remove duplicate #[test] if it exists again (just to be safe)
content = content.replace("    #[test]\n\n    #[test]\n", "    #[test]\n")

with open("crates/nes-core/src/experimental/spatial_bot.rs", "w") as f:
    f.write(content)
