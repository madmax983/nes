#!/bin/bash
cargo run -p nes-tui > tui.log 2>&1 &
sleep 5
cat tui.log
kill %1
