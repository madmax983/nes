use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use nes_netplay::{ClientMessage, ServerMessage};

#[derive(Debug, Clone)]
pub struct NetplayRuntimeConfig {
    pub relay_addr: String,
    pub room: String,
    pub player: u8,
    pub input_delay_frames: u32,
    pub max_rollback_frames: u32,
    pub hash_check_every_frames: u64,
}

pub struct NetplayClient {
    tx: Sender<ClientMessage>,
    rx: Receiver<ServerMessage>,
    err_rx: Receiver<String>,
}

impl NetplayClient {
    pub fn connect(config: &NetplayRuntimeConfig) -> Result<Self, String> {
        if !matches!(config.player, 1 | 2) {
            return Err(format!(
                "netplay player must be 1 or 2, got {}",
                config.player
            ));
        }

        let stream = TcpStream::connect(&config.relay_addr)
            .map_err(|err| format!("failed to connect to relay {}: {err}", config.relay_addr))?;
        stream
            .set_nodelay(true)
            .map_err(|err| format!("failed to configure relay socket: {err}"))?;
        let writer_stream = stream
            .try_clone()
            .map_err(|err| format!("failed to clone relay socket writer: {err}"))?;
        let reader_stream = stream
            .try_clone()
            .map_err(|err| format!("failed to clone relay socket reader: {err}"))?;

        let (tx, write_rx) = mpsc::channel::<ClientMessage>();
        let (read_tx, rx) = mpsc::channel::<ServerMessage>();
        let (err_tx, err_rx) = mpsc::channel::<String>();

        let join_room = config.room.clone();
        let join_player = config.player;
        let writer_err_tx = err_tx.clone();
        thread::spawn(move || {
            if let Err(err) = writer_loop(writer_stream, write_rx, join_room, join_player) {
                let _ = writer_err_tx.send(err);
            }
        });

        let reader_err_tx = err_tx.clone();
        thread::spawn(move || {
            if let Err(err) = reader_loop(reader_stream, read_tx) {
                let _ = reader_err_tx.send(err);
            }
        });

        Ok(Self { tx, rx, err_rx })
    }

    pub fn send_input(&self, frame: u64, bits: u8) -> Result<(), String> {
        self.tx
            .send(ClientMessage::Input { frame, bits })
            .map_err(|err| format!("failed to queue netplay input: {err}"))
    }

    pub fn send_hash(&self, frame: u64, state_hash: u64) -> Result<(), String> {
        self.tx
            .send(ClientMessage::Hash { frame, state_hash })
            .map_err(|err| format!("failed to queue netplay hash: {err}"))
    }

    pub fn send_ping(&self, nonce: u64) -> Result<(), String> {
        self.tx
            .send(ClientMessage::Ping { nonce })
            .map_err(|err| format!("failed to queue netplay ping: {err}"))
    }

    pub fn try_recv(&self) -> Result<Option<ServerMessage>, String> {
        if let Some(err) = self.take_error() {
            return Err(err);
        }
        match self.rx.try_recv() {
            Ok(message) => Ok(Some(message)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err("netplay relay reader disconnected".to_owned()),
        }
    }

    fn take_error(&self) -> Option<String> {
        self.err_rx.try_recv().ok()
    }
}

fn writer_loop(
    mut stream: TcpStream,
    rx: Receiver<ClientMessage>,
    room: String,
    player: u8,
) -> Result<(), String> {
    write_message(&mut stream, &ClientMessage::Join { room, player })?;

    loop {
        match rx.recv_timeout(Duration::from_millis(10)) {
            Ok(message) => write_message(&mut stream, &message)?,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}

fn reader_loop(stream: TcpStream, tx: Sender<ServerMessage>) -> Result<(), String> {
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|err| format!("failed to read relay message: {err}"))?;
        if bytes == 0 {
            return Err("relay closed connection".to_owned());
        }
        let message = serde_json::from_str::<ServerMessage>(line.trim())
            .map_err(|err| format!("failed to decode relay message: {err}"))?;
        tx.send(message)
            .map_err(|err| format!("failed to deliver relay message to main loop: {err}"))?;
    }
}

fn write_message(stream: &mut TcpStream, message: &ClientMessage) -> Result<(), String> {
    let line = serde_json::to_string(message)
        .map_err(|err| format!("failed to encode relay message: {err}"))?;
    stream
        .write_all(line.as_bytes())
        .and_then(|_| stream.write_all(b"\n"))
        .and_then(|_| stream.flush())
        .map_err(|err| format!("failed to write relay message: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn writer_loop_survives_idle_timeout_and_sends_late_message() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        let client_stream = TcpStream::connect(addr).expect("connect client");
        let (server_stream, _) = listener.accept().expect("accept server stream");
        server_stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .expect("set read timeout");
        let mut server_reader = BufReader::new(server_stream);

        let (tx, rx) = mpsc::channel::<ClientMessage>();
        let writer_thread =
            thread::spawn(move || writer_loop(client_stream, rx, "test-room".to_owned(), 1));

        let mut join_line = String::new();
        server_reader
            .read_line(&mut join_line)
            .expect("read join line");
        let join = serde_json::from_str::<ClientMessage>(join_line.trim()).expect("parse join");
        assert_eq!(
            join,
            ClientMessage::Join {
                room: "test-room".to_owned(),
                player: 1
            }
        );

        thread::sleep(Duration::from_millis(30));
        tx.send(ClientMessage::Input {
            frame: 99,
            bits: 0x81,
        })
        .expect("writer thread still alive after idle timeout");

        let mut input_line = String::new();
        server_reader
            .read_line(&mut input_line)
            .expect("read input line");
        let input = serde_json::from_str::<ClientMessage>(input_line.trim()).expect("parse input");
        assert_eq!(
            input,
            ClientMessage::Input {
                frame: 99,
                bits: 0x81
            }
        );

        drop(tx);
        let result = writer_thread.join().expect("join writer thread");
        assert!(result.is_ok(), "writer loop should exit cleanly");
    }

    #[test]
    fn connect_rejects_invalid_player_before_attempting_socket_connect() {
        let config = NetplayRuntimeConfig {
            relay_addr: "127.0.0.1:1".to_owned(),
            room: "test-room".to_owned(),
            player: 3,
            input_delay_frames: 2,
            max_rollback_frames: 16,
            hash_check_every_frames: 60,
        };
        let err = NetplayClient::connect(&config)
            .err()
            .expect("invalid player should fail fast");
        assert!(err.contains("player must be 1 or 2"));
    }

    #[test]
    fn send_methods_report_channel_disconnect_errors() {
        let (tx, rx) = mpsc::channel::<ClientMessage>();
        drop(rx);
        let (_server_tx, server_rx) = mpsc::channel::<ServerMessage>();
        let (_err_tx, err_rx) = mpsc::channel::<String>();
        let client = NetplayClient {
            tx,
            rx: server_rx,
            err_rx,
        };

        let input_err = client
            .send_input(10, 0x12)
            .expect_err("send_input should report disconnected writer");
        assert!(input_err.contains("failed to queue netplay input"));

        let hash_err = client
            .send_hash(20, 0xDEADBEEF)
            .expect_err("send_hash should report disconnected writer");
        assert!(hash_err.contains("failed to queue netplay hash"));

        let ping_err = client
            .send_ping(42)
            .expect_err("send_ping should report disconnected writer");
        assert!(ping_err.contains("failed to queue netplay ping"));
    }

    #[test]
    fn try_recv_returns_messages_and_prioritizes_error_channel() {
        let (tx, _write_rx) = mpsc::channel::<ClientMessage>();
        let (server_tx, server_rx) = mpsc::channel::<ServerMessage>();
        let (_err_tx, err_rx) = mpsc::channel::<String>();
        let client = NetplayClient {
            tx,
            rx: server_rx,
            err_rx,
        };

        server_tx
            .send(ServerMessage::Pong { nonce: 7 })
            .expect("sending pong should succeed");
        assert_eq!(
            client.try_recv().expect("try_recv should read message"),
            Some(ServerMessage::Pong { nonce: 7 })
        );
        assert_eq!(
            client.try_recv().expect("empty queue should return none"),
            None
        );
        drop(server_tx);
        let disconnected = client
            .try_recv()
            .expect_err("disconnected queue should propagate error");
        assert!(disconnected.contains("reader disconnected"));

        let (tx2, _write_rx2) = mpsc::channel::<ClientMessage>();
        let (_server_tx2, server_rx2) = mpsc::channel::<ServerMessage>();
        let (err_tx2, err_rx2) = mpsc::channel::<String>();
        err_tx2
            .send("relay-side failure".to_owned())
            .expect("error send should succeed");
        let client_with_err = NetplayClient {
            tx: tx2,
            rx: server_rx2,
            err_rx: err_rx2,
        };
        let err = client_with_err
            .try_recv()
            .expect_err("queued relay error should take precedence");
        assert_eq!(err, "relay-side failure");
        assert_eq!(client_with_err.take_error(), None);
    }

    #[test]
    fn reader_loop_forwards_messages_and_reports_relay_close() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        let client_stream = TcpStream::connect(addr).expect("connect client");
        let (mut server_stream, _) = listener.accept().expect("accept server stream");
        let (tx, rx) = mpsc::channel::<ServerMessage>();

        let reader_thread = thread::spawn(move || reader_loop(client_stream, tx));

        let payload =
            serde_json::to_string(&ServerMessage::Pong { nonce: 99 }).expect("serialize message");
        server_stream
            .write_all(payload.as_bytes())
            .and_then(|_| server_stream.write_all(b"\n"))
            .expect("write framed relay line");
        server_stream.flush().expect("flush relay line");
        drop(server_stream);

        let received = rx
            .recv_timeout(Duration::from_millis(250))
            .expect("reader should forward server message");
        assert_eq!(received, ServerMessage::Pong { nonce: 99 });

        let close_err = reader_thread
            .join()
            .expect("join reader thread")
            .expect_err("reader loop should report EOF as an error");
        assert!(close_err.contains("relay closed connection"));
    }
}
