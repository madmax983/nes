import subprocess
import os

crates = [
    "nes-core",
    "nes-ai",
    "nes-desktop",
    "nes-dsl",
    "nes-mcp",
    "nes-netplay",
    "nes-relay",
    "nes-rewind",
    "nes-test-harness",
    "nes-tui",
    "nes-web"
]

for crate in crates:
    print(f"Testing {crate}...")
    res = subprocess.run(f"cargo test -p {crate}", shell=True, capture_output=True, text=True)
    if res.returncode != 0:
        print(f"{crate} failed!")
        print(res.stderr)
        print(res.stdout)
    else:
        print(f"{crate} passed.")
