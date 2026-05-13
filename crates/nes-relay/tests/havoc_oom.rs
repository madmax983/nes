use std::net::TcpStream;
use std::thread;

// To test `nes-relay`, we can connect to the relay with a real TcpStream
// but send an infinite stream of A's with no newline to cause an OOM panic.
#[test]
#[ignore = "Havoc OOM Attack"]
fn havoc_relay_oom() {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_nes-relay"))
        .arg("--port")
        .arg("10005")
        .spawn()
        .expect("start relay");

    thread::sleep(std::time::Duration::from_millis(500));

    let mut stream = TcpStream::connect("127.0.0.1:10005").expect("connect to relay");

    // Write infinite data
    let buf = vec![b'A'; 1024 * 1024]; // 1MB chunks

    use std::io::Write;
    // Start writing in a loop
    for _ in 0..150 {
        // ~150MB
        if stream.write_all(&buf).is_err() {
            break;
        }
    }

    // We expect the relay to either crash, OOM, or at least memory grows unbounded.
    let _ = child.kill();
    let _ = child.wait();
}
