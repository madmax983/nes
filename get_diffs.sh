#!/bin/bash
cat crates/nes-desktop/src/main.rs | sed -n '66,95p'
echo "====="
cat crates/nes-desktop/src/main.rs | sed -n '734,736p'
echo "====="
cat crates/nes-desktop/src/main.rs | sed -n '1549,1675p'
echo "====="
cat crates/nes-desktop/src/main.rs | sed -n '1734,1746p'
