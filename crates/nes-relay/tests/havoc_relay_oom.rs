#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn havoc_crash_relay_daemon_oom() {
    // Start server normally on a random port to avoid port conflicts
    // It's tricky to get the exact port it bound to when it's `0`, so we use a known high port
    let port = 4547;
    let child = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "ulimit -v 50000 && exec {} --bind 127.0.0.1:{}",
            env!("CARGO_BIN_EXE_nes-relay"),
            port
        ))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn nes-relay daemon with memory limit");

    let mut guard = ChildGuard(child);

    // Give it a moment to start
    thread::sleep(Duration::from_millis(500));

    // The Kill Switch: The payload tells the relay daemon to buffer a massive
    // string without a newline character. `BufReader::read_line` has no internal limit.
    // We only need to send ~60MB to trigger the 50MB limit.
    let mut client = match TcpStream::connect(format!("127.0.0.1:{port}")) {
        Ok(c) => c,
        Err(e) => {
            let stderr = guard.0.stderr.take().expect("get stderr");
            let mut err_reader = BufReader::new(stderr);
            let mut err_line = String::new();
            let _ = err_reader.read_line(&mut err_line);
            panic!("Could not connect to relay. Error: {e}. Stderr: {err_line}");
        }
    };

    let chunk = vec![b' '; 1024 * 1024]; // 1MB
    for _ in 0..100 {
        // 100 MB max
        if client.write_all(&chunk).is_err() {
            break;
        }
    }
    let _ = client.write_all(b"\n");
    drop(client);

    let mut status = None;
    for _ in 0..100 {
        if let Ok(Some(s)) = guard.0.try_wait() {
            status = Some(s);
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    if let Some(status) = status {
        assert!(
            !status.success(),
            "Relay survived the OOM attack under memory limit. This should not happen."
        );
    } else {
        panic!("Relay survived the OOM attack and is still running.");
    }
}
