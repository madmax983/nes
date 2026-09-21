with open('crates/nes-core/src/experimental/mod.rs', 'r') as f:
    lines = f.readlines()

new_lines = []
skip = False
for i, line in enumerate(lines):
    if line.strip() == "pub mod code_coverage;":
        if i > 0 and "Tracks which CPU addresses have been executed to build a code coverage map." in lines[i-1]:
            if not skip:
                new_lines.append(lines[i-1])
                new_lines.append(line)
                skip = True
            else:
                new_lines.pop() # remove the duplicate comment
    elif not "Tracks which CPU addresses have been executed to build a code coverage map." in line:
        new_lines.append(line)

with open('crates/nes-core/src/experimental/mod.rs', 'w') as f:
    f.writelines(new_lines)
