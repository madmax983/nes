use nes_core::NesCore;
use nes_mcp::macro_engine::execute_macro_script;
use proptest::prelude::*;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

#[test]
#[should_panic(expected = "Havoc: system is vulnerable to DoS! The loop hung.")]
fn havoc_wait_causes_denial_of_service() {
    let script = "WAIT 18446744073709551615";
    let (tx, rx) = channel();

    thread::spawn(move || {
        let mut core = NesCore::new();
        let _ = execute_macro_script(&mut core, script);
        let _ = tx.send(());
    });

    let result = rx.recv_timeout(Duration::from_millis(100));
    assert!(
        result.is_ok(),
        "Havoc: system is vulnerable to DoS! The loop hung."
    );
}

// Proptest to fuzz for DoS vulnerabilities using random long WAIT commands.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    #[should_panic(expected = "Expected random long waits to finish quickly!")]
    fn havoc_fuzz_macro(
        // Generates large numbers for WAIT (between 100,000,000,000,000,000 and u64::MAX)
        // using proptest
        count in 100_000_000_000_000_000u64..u64::MAX
    ) {
        let (tx, rx) = channel();
        let script = format!("WAIT {}", count);

        thread::spawn(move || {
            let mut core = NesCore::new();
            let _ = execute_macro_script(&mut core, &script);
            let _ = tx.send(());
        });

        let result = rx.recv_timeout(Duration::from_millis(50));
        assert!(
            result.is_ok(),
            "Expected random long waits to finish quickly!"
        );
    }
}
