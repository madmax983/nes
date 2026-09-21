#!/bin/bash
cargo test -p nes-core -- test_chr_regions_no_allocation
cargo test -p nes-core -- test_chr_regions_no_allocation_mode_0
cargo test -p nes-core -- test_chr_regions_no_allocation_mode_1
cargo test -p nes-core -- test_chr_regions_no_allocation_mode_2
