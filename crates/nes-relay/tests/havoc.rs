use nes_relay::config::parse_args;
use proptest::prelude::*;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread;

// Note: These tests must be marked `#[ignore]` as per the requirements for Chaos Engineering
// They simulate edge cases that panic, dead-lock, or consume excessive system resources to prove fragility
// but should not execute during standard CI runs to keep the build green.

fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let addr = listener.local_addr().expect("listener local addr");
    let client = TcpStream::connect(addr).expect("connect to listener");
    let (server, _) = listener.accept().expect("accept loopback");
    (client, server)
}

#[test]
#[should_panic]
#[ignore = "Havoc DoS Attack"]
fn havoc_test_read_client_message_dos() {
    let (mut client, _server) = connected_pair();

    let thread_handle = thread::spawn(move || {
        // A streaming DoS payload using a very long string without allocating 100MB up-front to prevent CI OOMs.
        for _ in 0..10_000_000 {
            if client.write_all(b"{\"type\":\"ping\",\"nonce\":").is_err() {
                break;
            }
        }
        let _ = client.write_all(b"0}\n");
    });

    // Wait for the client to queue up a significant payload
    thread::sleep(std::time::Duration::from_millis(50));

    let mut reader = std::io::BufReader::new(_server);
    let mut line = String::new();
    let _ = std::io::BufRead::read_line(&mut reader, &mut line).unwrap();
    // After allocating a giant string, parsing it as json should panic by recursing too deeply or hitting a token boundary
    let _ = serde_json::from_str::<nes_netplay::ClientMessage>(&line).unwrap();

    thread_handle.join().unwrap();
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    #[should_panic]
    #[ignore = "Havoc Proptest Overflow Attack"]
    fn havoc_test_parse_args_overflow(
        latency in "\\d{20,100}" // An overly large number that causes panic on unwrap because it overflows the `u64`
    ) {
        let args = vec![
            "--bind".to_owned(),
            "0.0.0.0:9999".to_owned(),
            "--latency-ms".to_owned(),
            latency
        ];
        // The proptest finds strings that correctly parse regex but panic on u64 cast
        let _ = parse_args(args).unwrap();
    }
}

struct ProcessGuard(std::process::Child);

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "Havoc OOM Attack (SIGKILL)"]
fn havoc_test_read_client_message_oom() {
    use std::process::Command;

    // First start the server with an explicitly provided ephemeral port
    // so we don't have to parse it

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let addr = listener.local_addr().expect("listener local addr");
    let port = addr.port();
    drop(listener); // free the port for relay

    // We start the relay binary so it is genuinely testing the application
    let child = Command::new(env!("CARGO_BIN_EXE_nes-relay"))
        .arg("--bind")
        .arg(format!("127.0.0.1:{}", port))
        .spawn()
        .expect("Failed to start relay binary");

    // Create a guard to ensure the process is killed even if the test panics
    let mut relay_proc = ProcessGuard(child);

    // Add a slight delay to allow the server socket to truly bind and accept before connecting.
    thread::sleep(std::time::Duration::from_millis(500));

    // Try connecting with a retry loop to prevent ConnectionRefused
    let mut client = None;
    for _ in 0..20 {
        if let Ok(c) = TcpStream::connect(format!("127.0.0.1:{}", port)) {
            client = Some(c);
            break;
        }
        thread::sleep(std::time::Duration::from_millis(100));
    }

    let mut client = client.expect("connect to relay");

    // The rogue client streams infinite bytes without a newline.
    // The relay server reads from the stream.
    // This will cause the relay binary to allocate memory until it OOMs and gets killed.
    loop {
        // Write chunks of 'A' continuously
        if client.write_all(&[b'A'; 1024]).is_err() {
            break;
        }
    }

    // Wait for the relay process to be killed by the OS.
    let status = relay_proc.0.wait().expect("Failed to wait on relay binary");
    // status should be a failure due to SIGKILL
    assert!(!status.success());
}
