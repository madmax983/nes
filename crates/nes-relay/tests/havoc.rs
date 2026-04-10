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
