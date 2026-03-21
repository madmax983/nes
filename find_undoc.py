import os
import re

def check_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    lines = content.split('\n')
    undoc_items = []

    for i, line in enumerate(lines):
        if re.match(r'^\s*pub\s+(fn|struct|enum|trait)\s+', line):
            # Check if previous lines have docs
            if i > 0 and not lines[i-1].strip().startswith('///') and not lines[i-1].strip().startswith('#['):
                undoc_items.append((i+1, line.strip()))

    if undoc_items:
        print(f"File: {filepath}")
        for line_num, item in undoc_items:
            print(f"  Line {line_num}: {item}")

for root, dirs, files in os.walk('crates'):
    for file in files:
        if file.endswith('.rs'):
            check_file(os.path.join(root, file))
