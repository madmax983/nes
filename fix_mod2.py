with open('crates/nes-core/src/experimental/mod.rs', 'r') as f:
    lines = f.readlines()

new_lines = []
for i, line in enumerate(lines):
    if line.strip() == "/// Tracks which CPU addresses have been executed to build a code coverage map.":
        new_lines.append(line)
        new_lines.append("pub mod code_coverage;\n")
    else:
        new_lines.append(line)

with open('crates/nes-core/src/experimental/mod.rs', 'w') as f:
    f.writelines(new_lines)
