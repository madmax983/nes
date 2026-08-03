import re

with open('target/llvm-cov/html/coverage/app/crates/nes-dsl/src/parser.rs.html', 'r') as f:
    html = f.read()

for i, line in enumerate(html.split('\n')):
    if 'unexecuted-line' in line:
        print(f"L{i}: {line}")
