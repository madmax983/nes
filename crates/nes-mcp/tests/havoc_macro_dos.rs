#[test]
fn havoc_crash_mcp_dos_wait_frames() {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut core = nes_core::NesCore::new();
        // The trigger: A wait command with u64::MAX will hang the thread forever.
        let script = "WAIT 18446744073709551615";
        let _ = nes_mcp::macro_engine::execute_macro_script(&mut core, script, None);
        tx.send(()).unwrap();
    });

    // Wait for up to 1 second. If it doesn't finish, we've successfully proven the DoS.
    rx.recv_timeout(Duration::from_millis(500))
        .expect("timeout");
}
