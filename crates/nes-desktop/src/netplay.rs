use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use nes_netplay::{ClientMessage, ServerMessage};

/// Configuration settings to launch a netplay session.
///
/// This tells the `NetplayClient` which relay server to connect to,
/// the room ID, the local player index, and how to handle rollback mechanics.
///
/// ## Examples
///
/// ```
/// use nes_desktop::netplay::NetplayRuntimeConfig;
///
/// let config = NetplayRuntimeConfig {
///     relay_addr: "127.0.0.1:3555".to_string(),
///     room: "room_123".to_string(),
///     player: 1,
///     input_delay_frames: 2,
///     max_rollback_frames: 60,
///     hash_check_every_frames: 60,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct NetplayRuntimeConfig {
    pub relay_addr: String,
    pub room: String,
    pub player: u8,
    pub input_delay_frames: u32,
    pub max_rollback_frames: u32,
    pub hash_check_every_frames: u64,
}

/// An asynchronous TCP client that bridges local emulation with the `nes-relay` server.
///
/// Emulation is a synchronous, cycle-accurate beast. Networks are asynchronous and
/// chaotic. `NetplayClient` solves this by spinning up an I/O thread. It exposes
/// simple, non-blocking MPSC channels so the main emulator loop can shove inputs
/// into the ether and pull down remote inputs without ever dropping a frame.
///
/// ## Examples
///
/// ```no_run
/// use nes_desktop::netplay::{NetplayClient, NetplayRuntimeConfig};
///
/// let config = NetplayRuntimeConfig {
///     relay_addr: "127.0.0.1:3555".to_string(),
///     room: "match_xyz".to_string(),
///     player: 1,
///     input_delay_frames: 2,
///     max_rollback_frames: 60,
///     hash_check_every_frames: 60,
/// };
/// let client = NetplayClient::connect(&config).unwrap();
/// client.send_ping(1234).unwrap();
/// ```
pub struct NetplayClient {
    tx: Sender<ClientMessage>,
    rx: Receiver<ServerMessage>,
    err_rx: Receiver<String>,
}

impl NetplayClient {
    /// Attempts to establish a connection to the configured `nes-relay` server.
    ///
    /// Why do we spawn a background thread? Emulation loops are incredibly sensitive to
    /// timing. If we blocked the main thread waiting for a TCP response, the emulator
    /// would stutter and drop audio. By spinning up a background I/O thread, we can
    /// shove inputs over the network asynchronously while keeping the 60 FPS dream alive.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use nes_desktop::netplay::{NetplayClient, NetplayRuntimeConfig};
    ///
    /// let config = NetplayRuntimeConfig {
    ///     relay_addr: "127.0.0.1:4545".to_string(),
    ///     room: "my_room".to_string(),
    ///     player: 1,
    ///     input_delay_frames: 2,
    ///     max_rollback_frames: 30,
    ///     hash_check_every_frames: 60,
    /// };
    ///
    /// let client = NetplayClient::connect(&config).expect("Relay is down!");
    /// ```
    ///
    /// ## Panics
    ///
    /// This function does not panic, but it returns an error if the relay server is unreachable.
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

    /// Queues an input event to be sent to the remote peer via the relay.
    ///
    /// This is the lifeblood of netplay. We send our local `frame` and controller `bits`
    /// to the server, which then forwards them to Player 2. If we don't send this,
    /// Player 2's emulator will eventually guess wrong and trigger a massive rollback.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use nes_desktop::netplay::{NetplayClient, NetplayRuntimeConfig};
    /// # let config = NetplayRuntimeConfig {
    /// #     relay_addr: "127.0.0.1:4545".to_string(), room: "room".to_string(), player: 1,
    /// #     input_delay_frames: 2, max_rollback_frames: 30, hash_check_every_frames: 60,
    /// # };
    /// # let client = NetplayClient::connect(&config).unwrap();
    /// // Frame 120, holding 'Right' (0x01)
    /// client.send_input(120, 0x01).unwrap();
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns an error if the underlying network channel has been disconnected or broken.
    pub fn send_input(&self, frame: u64, bits: u8) -> Result<(), String> {
        self.tx
            .send(ClientMessage::Input { frame, bits })
            .map_err(|err| format!("failed to queue netplay input: {err}"))
    }

    /// Sends a local emulator state hash to the remote peer to verify sync.
    ///
    /// Rollback netcode is an illusion of perfection. Occasionally, the emulators diverge
    /// due to a bug or bad guess. By periodically sending a snapshot hash, we can
    /// detect desyncs early before the players notice they are playing two entirely
    /// different realities.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use nes_desktop::netplay::{NetplayClient, NetplayRuntimeConfig};
    /// # let config = NetplayRuntimeConfig {
    /// #     relay_addr: "127.0.0.1:4545".to_string(), room: "room".to_string(), player: 1,
    /// #     input_delay_frames: 2, max_rollback_frames: 30, hash_check_every_frames: 60,
    /// # };
    /// # let client = NetplayClient::connect(&config).unwrap();
    /// // Frame 60, computed state hash
    /// client.send_hash(60, 0x1A2B3C4D).unwrap();
    /// ```
    pub fn send_hash(&self, frame: u64, state_hash: u64) -> Result<(), String> {
        self.tx
            .send(ClientMessage::Hash { frame, state_hash })
            .map_err(|err| format!("failed to queue netplay hash: {err}"))
    }

    /// Sends a ping to measure Round-Trip Time (RTT) to the relay.
    ///
    /// Why do we ping? Because the internet is a series of tubes, and sometimes those tubes
    /// get clogged. Measuring RTT allows the emulator to dynamically adjust the
    /// `input_delay_frames` to keep the gameplay smooth under high latency.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use nes_desktop::netplay::{NetplayClient, NetplayRuntimeConfig};
    /// # let config = NetplayRuntimeConfig {
    /// #     relay_addr: "127.0.0.1:4545".to_string(), room: "room".to_string(), player: 1,
    /// #     input_delay_frames: 2, max_rollback_frames: 30, hash_check_every_frames: 60,
    /// # };
    /// # let client = NetplayClient::connect(&config).unwrap();
    /// // Send ping #1
    /// client.send_ping(1).unwrap();
    /// ```
    pub fn send_ping(&self, nonce: u64) -> Result<(), String> {
        self.tx
            .send(ClientMessage::Ping { nonce })
            .map_err(|err| format!("failed to queue netplay ping: {err}"))
    }

    /// Non-blockingly drains the next available message from the relay server.
    ///
    /// This is called exactly once per emulation frame. It grabs any queued inputs
    /// from the background thread. If we used a blocking read here, the game would
    /// literally stop and wait for the network.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use nes_desktop::netplay::{NetplayClient, NetplayRuntimeConfig};
    /// # let config = NetplayRuntimeConfig {
    /// #     relay_addr: "127.0.0.1:4545".to_string(), room: "room".to_string(), player: 1,
    /// #     input_delay_frames: 2, max_rollback_frames: 30, hash_check_every_frames: 60,
    /// # };
    /// # let client = NetplayClient::connect(&config).unwrap();
    /// if let Ok(Some(msg)) = client.try_recv() {
    ///     println!("Got message from relay!");
    /// }
    /// ```
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

/// Tracks the health of a live rollback netplay session.
///
/// The goal of netplay is the illusion of local multiplayer. This struct acts as the
/// pulse monitor, recording how often the illusion breaks (rollbacks), how far time
/// had to rewind (`max_rollback_distance`), and how much the network is struggling (`rtt_ms`, `jitter_ms`).
/// These metrics are vital for dynamically tuning `input_delay_frames` mid-game to hide latency.
pub struct NetplayRuntimeStats {
    pub latest_rtt_ms: Option<f64>,
    pub jitter_ms: f64,
    pub rollback_count: u64,
    pub max_rollback_distance: u64,
    pub desync_count: u64,
    pub input_delay_frames: u32,
}

impl NetplayRuntimeStats {
    /// Initializes tracking metrics, seeding the initial `input_delay_frames`.
    pub fn new(input_delay_frames: u32) -> Self {
        Self {
            latest_rtt_ms: None,
            jitter_ms: 0.0,
            rollback_count: 0,
            max_rollback_distance: 0,
            desync_count: 0,
            input_delay_frames,
        }
    }

    /// Records an observed Round-Trip Time sample to calculate average latency and jitter.
    pub fn observe_rtt_ms(&mut self, rtt_ms: f64) {
        if let Some(previous) = self.latest_rtt_ms {
            let delta = (rtt_ms - previous).abs();
            if self.jitter_ms <= f64::EPSILON {
                self.jitter_ms = delta;
            } else {
                // RFC3550-style EWMA jitter estimator.
                self.jitter_ms += (delta - self.jitter_ms) * 0.125;
            }
        }
        self.latest_rtt_ms = Some(rtt_ms);
    }

    /// Logs that a rollback occurred and updates the maximum rollback distance seen.
    pub fn observe_rollback(&mut self, distance: u64) {
        if distance == 0 {
            return;
        }
        self.rollback_count = self.rollback_count.saturating_add(1);
        self.max_rollback_distance = self.max_rollback_distance.max(distance);
    }

    /// Tallies a catastrophic timeline divergence.
    ///
    /// Desyncs happen when Player 1 and Player 2 calculate different state hashes for the same frame.
    /// It means the emulators have disagreed on reality. We track this to alert the players
    /// that they are no longer playing the same game.
    pub fn observe_desync(&mut self) {
        self.desync_count = self.desync_count.saturating_add(1);
    }

    /// Computes the last known round-trip time, defaulting to zero if unknown.
    ///
    /// We use this zero fallback to keep the dynamic delay logic mathematically
    /// safe during the initial connection handshake before the first ping returns.
    pub fn latest_rtt_ms_or_zero(&self) -> f64 {
        self.latest_rtt_ms.unwrap_or(0.0)
    }
}

/// Identifies which player's gamepad state belongs to the local user.
pub fn compute_local_netplay_bits(gamepad_bits: [u8; 2], local_player: u8) -> u8 {
    let local_slot = usize::from(local_player.saturating_sub(1));
    gamepad_bits.get(local_slot).copied().unwrap_or_else(|| {
        gamepad_bits
            .iter()
            .copied()
            .find(|bits| *bits != 0)
            .unwrap_or(0)
    })
}

/// Determines whether it is time to transmit a state hash based on the interval.
pub fn should_send_netplay_hash(hash_check_every: u64, frame: u64) -> bool {
    hash_check_every != 0 && frame != 0 && frame.is_multiple_of(hash_check_every)
}

/// Evaluates if enough time has passed to send another network ping to the relay.
pub fn schedule_netplay_ping(
    now: std::time::Instant,
    next_ping_at: &mut std::time::Instant,
    ping_nonce: &mut u64,
    pending_pings: &mut std::collections::BTreeMap<u64, std::time::Instant>,
    ping_interval: Duration,
    max_pending: usize,
) -> Option<u64> {
    if now < *next_ping_at {
        return None;
    }

    let nonce = *ping_nonce;
    *ping_nonce = ping_nonce.wrapping_add(1);
    pending_pings.insert(nonce, now);
    while pending_pings.len() > max_pending {
        pending_pings.pop_first();
    }
    *next_ping_at = now + ping_interval;
    Some(nonce)
}

/// Processes an incoming message from the `nes-relay` server, updating rollback state and metrics.
pub struct NetplayContext<'a> {
    pub netplay_client: Option<&'a NetplayClient>,
    pub rollback_engine: &'a mut nes_netplay::RollbackEngine,
    pub core: &'a mut nes_core::NesCore,
    pub netplay_local_player: u8,
    pub netplay_stats: &'a mut Option<NetplayRuntimeStats>,
    pub now: std::time::Instant,
    pub netplay_next_ping_at: &'a mut std::time::Instant,
    pub netplay_ping_nonce: &'a mut u64,
    pub netplay_pending_pings: &'a mut std::collections::BTreeMap<u64, std::time::Instant>,
    pub netplay_hash_check_every: u64,
}

pub fn poll_netplay_client_and_advance_frame(ctx: &mut NetplayContext<'_>) -> Result<(), String> {
    if let Some(client) = ctx.netplay_client {
        if let Some(nonce) = schedule_netplay_ping(
            ctx.now,
            ctx.netplay_next_ping_at,
            ctx.netplay_ping_nonce,
            ctx.netplay_pending_pings,
            super::NETPLAY_PING_INTERVAL,
            128,
        ) {
            client.send_ping(nonce)?;
        }

        while let Some(message) = client.try_recv()? {
            handle_netplay_server_message(
                message,
                ctx.rollback_engine,
                ctx.netplay_local_player,
                ctx.netplay_stats,
                ctx.netplay_pending_pings,
            )?;
        }
    }

    let step = ctx
        .rollback_engine
        .advance_frame(ctx.core)
        .map_err(|e| e.to_string())?;

    if step.rollback_distance > 0 {
        eprintln!(
            "[netplay] rollback={} frame={} local={:02X} remote={:02X}",
            step.rollback_distance, step.frame, step.local_bits, step.remote_bits
        );
        if let Some(stats) = ctx.netplay_stats.as_mut() {
            stats.observe_rollback(step.rollback_distance);
        }
    }

    let current_delay = ctx.rollback_engine.input_delay_frames();
    let max_auto_delay = ctx.rollback_engine.max_rollback_frames().clamp(
        super::NETPLAY_AUTO_DELAY_MIN_FRAMES,
        super::NETPLAY_AUTO_DELAY_MAX_FRAMES,
    );
    let target_delay = if let Some(stats) = ctx.netplay_stats.as_ref() {
        super::recommended_input_delay_frames(
            stats.latest_rtt_ms,
            stats.jitter_ms,
            super::NETPLAY_AUTO_DELAY_MIN_FRAMES,
            max_auto_delay,
            current_delay,
        )
    } else {
        current_delay
    };
    if target_delay != current_delay {
        ctx.rollback_engine
            .set_input_delay_frames(target_delay)
            .map_err(|e| e.to_string())?;
        if let Some(stats) = ctx.netplay_stats.as_mut() {
            stats.input_delay_frames = target_delay;
            eprintln!(
                "[netplay] adaptive delay {} -> {} (rtt={:.1}ms jitter={:.1}ms)",
                current_delay,
                target_delay,
                stats.latest_rtt_ms_or_zero(),
                stats.jitter_ms
            );
        }
    } else if let Some(stats) = ctx.netplay_stats.as_mut() {
        stats.input_delay_frames = current_delay;
    }

    if should_send_netplay_hash(ctx.netplay_hash_check_every, step.frame)
        && let Some(client) = ctx.netplay_client
    {
        client.send_hash(step.frame, step.state_hash)?;
    }

    Ok(())
}

pub fn handle_netplay_server_message(
    message: ServerMessage,
    rollback_engine: &mut nes_netplay::RollbackEngine,
    netplay_local_player: u8,
    netplay_stats: &mut Option<NetplayRuntimeStats>,
    netplay_pending_pings: &mut std::collections::BTreeMap<u64, std::time::Instant>,
) -> Result<(), String> {
    match message {
        ServerMessage::PeerInput {
            player,
            frame,
            bits,
        } => {
            if player != netplay_local_player {
                let ingest = rollback_engine.ingest_remote_input(frame, bits);
                if ingest.rollback_queued {
                    eprintln!(
                        "[netplay] queued rollback from frame {} due to late remote input",
                        frame
                    );
                }
            }
        }
        ServerMessage::PeerHash {
            player,
            frame,
            state_hash,
        } => {
            if player != netplay_local_player {
                match rollback_engine.compare_remote_hash(frame, state_hash) {
                    nes_netplay::HashComparison::Match => {}
                    nes_netplay::HashComparison::Mismatch => {
                        eprintln!(
                            "[netplay] desync detected at frame {} (remote hash {:016X})",
                            frame, state_hash
                        );
                        if let Some(stats) = netplay_stats.as_mut() {
                            stats.observe_desync();
                        }
                    }
                    nes_netplay::HashComparison::PendingLocalFrame => {}
                }
            }
        }
        ServerMessage::Joined {
            room,
            player,
            peer_present,
        } => {
            println!(
                "[netplay] joined room '{}' as P{} (peer_present={})",
                room, player, peer_present
            );
        }
        ServerMessage::PeerJoined { player } => {
            println!("[netplay] peer joined as P{}", player);
        }
        ServerMessage::PeerLeft { player } => {
            println!("[netplay] peer left (P{})", player);
        }
        ServerMessage::Error { message } => {
            return Err(format!("[netplay] relay error: {message}"));
        }
        ServerMessage::Pong { nonce } => {
            if let Some(sent_at) = netplay_pending_pings.remove(&nonce) {
                let rtt_ms = sent_at.elapsed().as_secs_f64() * 1_000.0;
                if let Some(stats) = netplay_stats.as_mut() {
                    stats.observe_rtt_ms(rtt_ms);
                }
            }
        }
    }
    Ok(())
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

    fn sample_ines(mapper: u8, prg_banks: u8) -> Vec<u8> {
        let mut rom = vec![0; 16 + (prg_banks as usize) * 16384];
        rom[0..4].copy_from_slice(b"NES\x1A");
        rom[4] = prg_banks;
        rom[5] = 0; // 0 CHR banks (uses CHR RAM)
        rom[6] = (mapper << 4) & 0xF0;
        rom[7] = mapper & 0xF0;
        rom
    }

    use super::{
        NetplayRuntimeStats, compute_local_netplay_bits, handle_netplay_server_message,
        schedule_netplay_ping, should_send_netplay_hash,
    };

    #[test]
    fn netplay_helper_functions_choose_local_bits_and_hash_schedule() {
        assert_eq!(compute_local_netplay_bits([0x12, 0x34], 1), 0x12);
        assert_eq!(compute_local_netplay_bits([0x12, 0x34], 2), 0x34);
        assert_eq!(compute_local_netplay_bits([0x12, 0x34], 3), 0x12);
        assert_eq!(compute_local_netplay_bits([0, 0], 9), 0);

        assert!(!should_send_netplay_hash(0, 120));
        assert!(!should_send_netplay_hash(60, 0));
        assert!(should_send_netplay_hash(60, 120));
        assert!(!should_send_netplay_hash(60, 121));
    }

    #[test]
    fn schedule_netplay_ping_enforces_deadline_nonce_and_pending_cap() {
        let now = std::time::Instant::now();
        let mut next_ping_at = now + Duration::from_millis(10);
        let mut nonce = 5_u64;
        let mut pending = std::collections::BTreeMap::<u64, std::time::Instant>::new();
        pending.insert(1, now - Duration::from_millis(20));
        pending.insert(2, now - Duration::from_millis(10));

        assert_eq!(
            schedule_netplay_ping(
                now,
                &mut next_ping_at,
                &mut nonce,
                &mut pending,
                Duration::from_millis(500),
                2
            ),
            None
        );
        assert_eq!(nonce, 5);
        assert_eq!(pending.len(), 2);

        let due_now = now + Duration::from_millis(10);
        let scheduled = schedule_netplay_ping(
            due_now,
            &mut next_ping_at,
            &mut nonce,
            &mut pending,
            Duration::from_millis(500),
            2,
        );
        assert_eq!(scheduled, Some(5));
        assert_eq!(nonce, 6);
        assert_eq!(next_ping_at, due_now + Duration::from_millis(500));
        assert_eq!(pending.len(), 2, "pending set should be capped to max");
        assert!(
            !pending.contains_key(&1),
            "oldest nonce should be evicted first"
        );
        assert!(pending.contains_key(&5), "new nonce should be tracked");
    }

    #[test]
    fn handle_netplay_server_message_updates_stats_and_errors() {
        let mut core = nes_core::NesCore::new();
        let mut rom = sample_ines(0, 1);
        let prg_start = 16;
        rom[prg_start + 0x3FFC] = 0x00;
        rom[prg_start + 0x3FFD] = 0x80;
        core.load_ines_rom(&rom).expect("sample rom should load");

        let mut rollback_engine = nes_netplay::RollbackEngine::new(nes_netplay::RollbackConfig {
            local_player: 1,
            input_delay_frames: 2,
            max_rollback_frames: 16,
        })
        .expect("rollback config should be valid");
        let first_step = rollback_engine
            .advance_frame(&mut core)
            .expect("initial rollback step should succeed");

        let mut stats = Some(NetplayRuntimeStats::new(2));
        let mut pending = std::collections::BTreeMap::<u64, std::time::Instant>::new();
        pending.insert(7, std::time::Instant::now() - Duration::from_millis(10));

        handle_netplay_server_message(
            ServerMessage::Pong { nonce: 7 },
            &mut rollback_engine,
            1,
            &mut stats,
            &mut pending,
        )
        .expect("pong message should process");
        assert!(pending.is_empty());
        let rtt_ms = stats.as_ref().and_then(|s| s.latest_rtt_ms).unwrap_or(0.0);
        assert!(
            (1.0..=500.0).contains(&rtt_ms),
            "expected plausible RTT ms value, got {rtt_ms}"
        );

        handle_netplay_server_message(
            ServerMessage::PeerInput {
                player: 2,
                frame: first_step.frame,
                bits: 0x01,
            },
            &mut rollback_engine,
            1,
            &mut stats,
            &mut pending,
        )
        .expect("peer input should process");
        let rollback_step = rollback_engine
            .advance_frame(&mut core)
            .expect("post-input rollback step should succeed");
        assert!(
            rollback_step.rollback_distance > 0,
            "late remote input should queue rollback"
        );

        handle_netplay_server_message(
            ServerMessage::PeerHash {
                player: 2,
                frame: rollback_step.frame,
                state_hash: rollback_step.state_hash.wrapping_add(1),
            },
            &mut rollback_engine,
            1,
            &mut stats,
            &mut pending,
        )
        .expect("peer hash should process");
        assert_eq!(stats.as_ref().map_or(0, |s| s.desync_count), 1);

        let err = handle_netplay_server_message(
            ServerMessage::Error {
                message: "boom".to_owned(),
            },
            &mut rollback_engine,
            1,
            &mut stats,
            &mut pending,
        )
        .expect_err("relay errors should propagate");
        assert!(err.contains("relay error"));
    }
    #[test]
    fn poll_netplay_client_and_advance_frame_handles_missing_client_gracefully() {
        let mut core = nes_core::NesCore::default();
        let mut engine = nes_netplay::RollbackEngine::new(nes_netplay::RollbackConfig {
            max_rollback_frames: 4,
            input_delay_frames: 0,
            local_player: 1,
        })
        .unwrap();
        let now = std::time::Instant::now();
        let mut next_ping = now;
        let mut nonce = 0;
        let mut pings = std::collections::BTreeMap::new();
        let mut stats = None;
        let mut ctx = super::NetplayContext {
            netplay_client: None,
            rollback_engine: &mut engine,
            core: &mut core,
            netplay_local_player: 1,
            netplay_stats: &mut stats,
            now,
            netplay_next_ping_at: &mut next_ping,
            netplay_ping_nonce: &mut nonce,
            netplay_pending_pings: &mut pings,
            netplay_hash_check_every: 0,
        };
        let res = super::poll_netplay_client_and_advance_frame(&mut ctx);
        assert!(res.is_ok());
    }
}
