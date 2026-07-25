#!/bin/bash
set -e
cargo install cargo-tarpaulin || true
cargo tarpaulin --ignore-tests
