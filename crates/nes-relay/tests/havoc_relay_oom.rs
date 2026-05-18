use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

#[test]
#[ignore = "havoc target"]
fn havoc_test_relay_read_line_oom() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");

    // Start a server thread that reads lines from the client
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        // We set a small read timeout so it doesn't block forever if we don't send enough data
        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        // This read_line will attempt to read the entire payload because there is no newline.
        let _ = reader.read_line(&mut line);

        panic!(
            "OOM or buffer overflow: read_line consumed the entire massive payload without limits: len={}",
            line.len()
        );
    });

    // Client thread sending a massive stream without newlines
    let mut client = TcpStream::connect(addr).expect("connect");
    let chunk = vec![b'A'; 1024 * 1024 * 10]; // 10MB chunk
    let _ = client.write_all(&chunk);

    let res = server.join();
    match res {
        Err(err) => {
            // We expect it to panic
            println!("Server thread panicked as expected: {:?}", err);
        }
        Ok(_) => {
            panic!("Server thread did not panic!");
        }
    }
}
