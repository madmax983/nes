#!/bin/bash
while true; do
  cargo test -p nes-desktop > output.txt 2>&1
  if [ $? -ne 0 ]; then
     echo "Test failed!"
     break
  fi
  echo "Tests passed!"
  break
done
