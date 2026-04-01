#!/bin/bash
cargo llvm-cov --workspace --all-features --all-targets --lcov --output-path lcov.info > output.txt 2>&1
