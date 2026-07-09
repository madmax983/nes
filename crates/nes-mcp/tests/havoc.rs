use nes_core::NesCore;
use nes_mcp::macro_engine::execute_macro_script;
use nes_mcp::{ToolParams, dispatch_tool};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]
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

    // Wait for up to 1 second. If it doesn't finish, we've successfully proven the DoS.
    rx.recv_timeout(Duration::from_millis(500))
        .expect("timeout");
}

#[test]
#[should_panic(expected = "timeout")]
fn havoc_test_wait_macro_dos() {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut core = NesCore::new();
        // The trigger: A wait command with an enormous u64 will hang the thread forever.
        let script = "WAIT 18446744073709551610";
        let _ = execute_macro_script(&mut core, script, None);
        tx.send(()).unwrap();
    });

    // Wait for up to 1 second. If it doesn't finish, we've successfully proven the DoS.
    rx.recv_timeout(Duration::from_millis(500))
        .expect("timeout");
}
