import os
import subprocess

def run_cmd(cmd):
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    return result.stdout, result.stderr

def check_missing_docs():
    cmd = "RUSTDOCFLAGS='-W missing_docs -W rustdoc::missing_crate_level_docs' cargo doc --no-deps --document-private-items 2>&1"
    out, err = run_cmd(cmd)

    missing = []
    for line in out.split('\n') + err.split('\n'):
        if "missing documentation for" in line or "missing crate level documentation" in line:
            missing.append(line.strip())

    return missing

missing = check_missing_docs()
for m in missing:
    print(m)
