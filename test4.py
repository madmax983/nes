import re
import glob

html_file = 'target/llvm-cov/html/coverage/app/crates/nes-dsl/src/lib.rs.html'
with open(html_file, 'r') as f:
    lines = f.readlines()

# Find unexecuted lines for strip_comments
for i, line in enumerate(lines):
    if "unexecuted-line" in line:
        print("Unexecuted line:", line)
