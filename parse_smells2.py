import re

with open("crates/nes-desktop/src/main.rs", "r") as f:
    lines = f.readlines()

def print_func(name):
    in_func = False
    brace_count = 0
    for i, line in enumerate(lines):
        if not in_func:
            if re.search(r'fn\s+'+name+r'\s*\(', line):
                in_func = True
                brace_count = line.count('{') - line.count('}')
                print(f"{i+1}: {line.rstrip()}")
        else:
            brace_count += line.count('{') - line.count('}')
            print(f"{i+1}: {line.rstrip()}")
            if brace_count <= 0:
                break

print_func("tests::")
