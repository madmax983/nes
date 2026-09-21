#!/bin/bash
set -e

# Setup environment for llvm-cov
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov || true

# Run coverage using llvm-cov
cargo llvm-cov --lcov --output-path lcov.info -p nes-core --all-features
