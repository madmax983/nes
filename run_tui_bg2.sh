#!/bin/bash
cargo run -p nes-tui -- roms/homebrew/homebrew.nes > tui2.log 2>&1 &
sleep 5
cat tui2.log
kill %1
