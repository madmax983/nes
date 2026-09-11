#!/bin/bash
# Local ESP32-S3 QEMU boot test for the NES firmware.
# Mirrors .github/workflows/esp32s3-qemu.yml using the workspace-local
# QEMU and esptool (tallow's proven recipe).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
FW="$HERE/crates/nes-embassy/firmware/esp32-s3"
BUILD="$FW/build"
QEMU="$HOME/workspace/tooling/qemu-esp32/qemu/bin/qemu-system-xtensa"
export LD_LIBRARY_PATH="$HOME/workspace/tooling/qemu-esp32/libs/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export PATH="$HOME/.cargo/bin:$PATH"
source ~/export-esp.sh
export CARGO_HOME=~/workspace/esptools/cargo-nes

mkdir -p "$BUILD"
cd "$FW"

echo "=== building release firmware (qemu feature) ==="
cargo +esp build --release --features qemu --offline

echo "=== elf2image ==="
ELF="$FW/target/xtensa-esp32s3-none-elf/release/nes-esp32-s3"
PYTHONPATH="$HOME/workspace/esptools/py" python3 -m esptool --chip esp32s3 elf2image \
  --flash-mode dio --flash-freq 80m --flash-size 4MB \
  -o "$BUILD/nes-app.bin" "$ELF"

echo "=== assembling 4MiB flash at offset 0x0 ==="
python3 - "$BUILD/nes-app.bin" "$BUILD/qemu_flash.bin" <<'EOF'
import sys
app = open(sys.argv[1],'rb').read()
flash = bytearray(b'\xff' * (4*1024*1024))
flash[0:len(app)] = app
open(sys.argv[2],'wb').write(flash)
print(f"app {len(app)} bytes -> {sys.argv[2]}")
EOF

echo "=== booting in QEMU ==="
LOG="$BUILD/uart0.log"
timeout 180 "$QEMU" -nographic -machine esp32s3 \
  -drive file="$BUILD/qemu_flash.bin",if=mtd,format=raw \
  -serial file:"$LOG" -monitor none -no-reboot || true

echo "--- uart0.log ---"
cat "$LOG"
echo "-----------------"
for marker in "nes: boot" "nes: rom loaded" "nes: entering frame loop" "NES_ESP32S3_BOOT_OK" "nes: frame 60"; do
  if grep -q "$marker" "$LOG"; then echo "OK: $marker"; else echo "MISSING: $marker"; fi
done
if grep -qiE "panicked|panic:|exception|guru meditation" "$LOG"; then
  echo "FAULT SIGNATURE FOUND"; exit 1
fi
if grep -q "nes: frame 60" "$LOG" && grep -q "NES_ESP32S3_BOOT_OK" "$LOG"; then
  echo "BOOT TEST PASSED"
else
  echo "BOOT TEST FAILED"; exit 1
fi
