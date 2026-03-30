#[test]
#[should_panic(expected = "timeout")]
fn havoc_dos_macro_wait_hang() {
    use nes_core::NesCore;
    use nes_mcp::macro_engine::execute_macro_script;
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
