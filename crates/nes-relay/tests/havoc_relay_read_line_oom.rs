use std::io::Write;
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[test]
#[ignore]
fn havoc_relay_read_line_oom() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nes-relay"))
        .env("PORT", "10101")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start nes-relay");

    thread::sleep(Duration::from_millis(1500));

    let handle = thread::spawn(move || {
        let mut stream = TcpStream::connect("127.0.0.1:10101").expect("Failed to connect");
        let chunk = vec![b'A'; 1024 * 1024];
        loop {
            if stream.write_all(&chunk).is_err() {
                break;
            }
        }
    });

    let status = child.wait().expect("Failed to wait on child");
    handle.join().unwrap();

    assert!(!status.success(), "Process exited normally instead of crashing due to capacity overflow/OOM");
}
