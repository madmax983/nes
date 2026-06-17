use nes_core::NesCore;
use nes_mcp::macro_engine::execute_macro_script;
use nes_mcp::{ToolParams, dispatch_tool};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_mcp_macro(script in ".*") {
        let mut core = NesCore::new();
        let _ = execute_macro_script(&mut core, &script, None);
    }

    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_mcp_params(
        tool_name in ".*",
        keys in proptest::collection::vec(".*", 0..5),
        values in proptest::collection::vec(".*", 0..5),
    ) {
        let mut core = NesCore::new();
        let mut params = ToolParams::new();
        for (k, v) in keys.into_iter().zip(values.into_iter()) {
            params.insert(k, v);
        }
        let _ = dispatch_tool(&mut core, &tool_name, &params);
    }

    #[test]
    #[ignore = "havoc target"]
    fn havoc_test_dispatch_hex_overflow(hex in "[A-Fa-f0-9 ]{1000,5000}") {
        let mut core = NesCore::new();
        let mut params = ToolParams::new();
        params.insert("rom_hex".to_owned(), hex);
        let _ = dispatch_tool(&mut core, "load_rom", &params);
    }
}

#[test]
#[should_panic(expected = "timeout")]
fn havoc_crash_mcp_dos_wait_frames() {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut core = NesCore::new();
        // The trigger: A wait command with u64::MAX will hang the thread forever.
        let script = "WAIT 18446744073709551615";
        let _ = execute_macro_script(&mut core, script, None);
        tx.send(()).unwrap();
    });

    rx.recv_timeout(Duration::from_millis(500))
        .expect("timeout");
}

#[test]
#[should_panic(expected = "output state lock")]
#[ignore = "Havoc Target Attack: Panic in output mutex closure"]
fn havoc_test_poison_output_lock() {
    let t = std::thread::spawn(|| {
        nes_mcp::publish_frame_with(256, 240, |_| {
            panic!("Havoc lock poison panic");
        });
    });
    let _ = t.join();

    let _ = nes_mcp::frame_chunk(0);
}
