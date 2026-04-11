#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::{BufRead, BufReader, Cursor};
use serde_json::from_str;
use nes_netplay::ClientMessage;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = from_str::<ClientMessage>(s);
    }
});
