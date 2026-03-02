with open("crates/nes-netplay/Cargo.toml", "r") as f:
    data = f.read()

if "unexpected_cfgs" not in data:
    data += "\n[lints.rust]\nunexpected_cfgs = { level = \"warn\", check-cfg = ['cfg(loom)'] }\n"

with open("crates/nes-netplay/Cargo.toml", "w") as f:
    f.write(data)
