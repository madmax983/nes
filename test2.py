import re
import glob

html_file = 'target/llvm-cov/html/coverage/app/crates/nes-dsl/src/lib.rs.html'
with open(html_file, 'r') as f:
    lines = f.readlines()

# Find unexecuted lines for strip_comments
for i, line in enumerate(lines):
    if "fn strip_comments" in line:
        print("Found strip_comments around line", i)
        for j in range(i, min(i+30, len(lines))):
            if "skipped-line" in lines[j]:
                print("Skipped line:", lines[j])
            if "unexecuted-line" in lines[j]:
                print("Unexecuted line:", lines[j])
