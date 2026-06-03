with open("crates/nes-mcp/tests/havoc.rs", "r") as f:
    content = f.read()

content = content.replace("    let mut core = NesCore::new();\n    let mut params = ToolParams::new();\n    \n    // We actually use the vulnerability", "    // We actually use the vulnerability")
content = content.replace("    let fake_ptr = 0x1000 as *const u8;", "")

with open("crates/nes-mcp/tests/havoc.rs", "w") as f:
    f.write(content)
