import sys

def check_lines(filename, start, end):
    with open('lcov.info', 'r') as f:
        lines = f.readlines()

    in_file = False
    hits = {}
    for line in lines:
        if line.startswith('SF:'):
            if filename in line:
                in_file = True
            else:
                in_file = False
        elif in_file and line.startswith('DA:'):
            parts = line.strip().split(':')[1].split(',')
            lineno = int(parts[0])
            hit = int(parts[1])
            if start <= lineno <= end:
                hits[lineno] = hit

    for i in range(start, end + 1):
        if i in hits:
            print(f"Line {i}: hit {hits[i]}")
        else:
            print(f"Line {i}: not instrumented")

check_lines('crates/nes-desktop/src/main.rs', 830, 855)
check_lines('crates/nes-desktop/src/main.rs', 886, 886)
check_lines('crates/nes-desktop/src/main.rs', 900, 900)
check_lines('crates/nes-desktop/src/main.rs', 906, 906)
check_lines('crates/nes-desktop/src/main.rs', 914, 914)
check_lines('crates/nes-desktop/src/main.rs', 1002, 1002)
