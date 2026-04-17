import subprocess
import os

print(subprocess.check_output(["cargo", "llvm-cov", "--no-clean", "-p", "nes-desktop"], text=True))
