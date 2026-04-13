import subprocess

def run_cmd(cmd):
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    return result.stdout, result.stderr

cmd = "RUSTDOCFLAGS='-W missing_docs -W rustdoc::missing_crate_level_docs' cargo doc --no-deps --document-private-items"
out, err = run_cmd(cmd)

lines = (out + "\n" + err).split('\n')
for i, line in enumerate(lines):
    if "missing documentation for the crate" in line:
        print(f"Match found! Context:")
        start = max(0, i-5)
        end = min(len(lines), i+5)
        for j in range(start, end):
            print(f"  {lines[j]}")
        print("-" * 40)
