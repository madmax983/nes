import re
import glob

html_file = 'target/llvm-cov/html/coverage/app/crates/nes-dsl/src/lib.rs.html'
with open(html_file, 'r') as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if "unexecuted" in line:
        print(line)
