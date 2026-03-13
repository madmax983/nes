#![no_main]

use libfuzzer_sys::fuzz_target;
use nes_relay::parse_args;

fuzz_target!(|data: Vec<String>| {
    let _ = parse_args(data);
});
