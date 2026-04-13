import subprocess
import re

def run_cmd(cmd):
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    return result.stdout, result.stderr

cmd = "RUSTDOCFLAGS='-W rustdoc::missing_crate_level_docs' cargo doc --no-deps --document-private-items"
out, err = run_cmd(cmd)

lines = (out + "\n" + err).split('\n')
for i, line in enumerate(lines):
    if "missing documentation for the crate" in line:
        # The next line usually points to the file
        if i + 1 < len(lines):
            print(lines[i+1])
