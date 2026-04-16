import sys

with open("crates/nes-desktop/src/main.rs", "r") as f:
    lines = f.readlines()

new_lines = []
for line in lines:
    if "macro_rules! build_ctx {" in line:
        if new_lines[-1] != "\n":
            new_lines.append("\n")
    new_lines.append(line)

with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.writelines(new_lines)
