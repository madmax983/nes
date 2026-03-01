use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nes_netplay::{ClientMessage, ServerMessage};

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:4545";
const DEFAULT_LATENCY_MS: u64 = 0;
const DEFAULT_JITTER_MS: u64 = 0;
const DEFAULT_LOSS_PCT: u8 = 0;
const DEFAULT_REORDER_PCT: u8 = 0;

#[derive(Default)]
struct RelayState {
    rooms: HashMap<String, RoomState>,
}

#[derive(Default)]
struct RoomState {
    players: HashMap<u8, Sender<ServerMessage>>,
}

#[derive(Debug, Clone, Copy)]
struct LinkCondition {
    latency_ms: u64,
    jitter_ms: u64,
    loss_pct: u8,
    reorder_pct: u8,
}

impl Default for LinkCondition {
    fn default() -> Self {
        Self {
            latency_ms: DEFAULT_LATENCY_MS,
            jitter_ms: DEFAULT_JITTER_MS,
            loss_pct: DEFAULT_LOSS_PCT,
            reorder_pct: DEFAULT_REORDER_PCT,
        }
    }
}

#[derive(Debug, Clone)]
struct RelayArgs {
    bind_addr: String,
    link: LinkCondition,
}

struct RelayNetSim {
    link: LinkCondition,
    rng: AtomicU64,
}

impl RelayNetSim {
    fn new(link: LinkCondition) -> Self {
        Self {
            link,
            rng: AtomicU64::new(seed_entropy()),
        }
    }

    fn should_drop(&self) -> bool {
        self.percent_hit(self.link.loss_pct)
    }

    fn sample_delay_ms(&self) -> u64 {
        let mut delay_ms = self.link.latency_ms as i64;
        if self.link.jitter_ms > 0 {
            let span = self.link.jitter_ms.saturating_mul(2).saturating_add(1);
            let offset = (self.next_u64() % span) as i64 - self.link.jitter_ms as i64;
            delay_ms = delay_ms.saturating_add(offset);
        }
        if self.percent_hit(self.link.reorder_pct) {
            // Reordering is modeled by adding additional variable delay to a subset of packets.
            let extra = self.link.latency_ms.max(1) + (self.next_u64() % (self.link.jitter_ms + 1));
            let extra_i64 = i64::try_from(extra).unwrap_or(i64::MAX);
            delay_ms = delay_ms.saturating_add(extra_i64);
        }
        if delay_ms <= 0 { 0 } else { delay_ms as u64 }
    }

    fn percent_hit(&self, pct: u8) -> bool {
        match pct {
            0 => false,
            100 => true,
            value => (self.next_u64() % 100) < u64::from(value),
        }
    }

    fn next_u64(&self) -> u64 {
        let mut prev = self.rng.load(Ordering::Relaxed);
        loop {
            let mut x = prev;
            if x == 0 {
                x = 0x9E37_79B9_7F4A_7C15;
            }
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            match self
                .rng
                .compare_exchange_weak(prev, x, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => return x,
                Err(observed) => prev = observed,
            }
        }
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args(std::env::args().skip(1).collect())?;
    let listener = TcpListener::bind(&args.bind_addr)
        .map_err(|err| format!("failed to bind {}: {err}", args.bind_addr))?;
    println!(
        "nes-relay listening on {} (latency={}ms jitter={}ms loss={}%% reorder={}%%)",
        args.bind_addr,
        args.link.latency_ms,
        args.link.jitter_ms,
        args.link.loss_pct,
        args.link.reorder_pct
    );

    let state = Arc::new(Mutex::new(RelayState::default()));
    let net_sim = Arc::new(RelayNetSim::new(args.link));
    for accepted in listener.incoming() {
        let stream = match accepted {
            Ok(stream) => stream,
            Err(err) => {
                eprintln!("accept failed: {err}");
                continue;
            }
        };
        let peer = stream
            .peer_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|_| "<unknown-peer>".to_owned());
        let shared = Arc::clone(&state);
        let net = Arc::clone(&net_sim);
        thread::spawn(move || {
            if let Err(err) = handle_client(stream, shared, net) {
                eprintln!("client {peer} disconnected with error: {err}");
            }
        });
    }
    Ok(())
}

fn parse_args(args: Vec<String>) -> Result<RelayArgs, String> {
    let mut parsed = RelayArgs {
        bind_addr: DEFAULT_BIND_ADDR.to_owned(),
        link: LinkCondition::default(),
    };
    let mut idx = 0_usize;
    while idx < args.len() {
        let arg = &args[idx];
        if arg == "--help" || arg == "-h" {
            return Err(format!(
                "Usage: nes-relay [--bind <addr>] [--latency-ms <n>] [--jitter-ms <n>] [--loss-pct <0..100>] [--reorder-pct <0..100>]\nDefault bind: {DEFAULT_BIND_ADDR}"
            ));
        }
        if arg == "--bind" {
            let Some(value) = args.get(idx + 1) else {
                return Err("missing value after --bind".to_owned());
            };
            parsed.bind_addr = value.clone();
            idx += 2;
            continue;
        }
        if arg == "--latency-ms" {
            let Some(value) = args.get(idx + 1) else {
                return Err("missing value after --latency-ms".to_owned());
            };
            parsed.link.latency_ms = parse_u64_arg(value, "--latency-ms")?;
            idx += 2;
            continue;
        }
        if arg == "--jitter-ms" {
            let Some(value) = args.get(idx + 1) else {
                return Err("missing value after --jitter-ms".to_owned());
            };
            parsed.link.jitter_ms = parse_u64_arg(value, "--jitter-ms")?;
            idx += 2;
            continue;
        }
        if arg == "--loss-pct" {
            let Some(value) = args.get(idx + 1) else {
                return Err("missing value after --loss-pct".to_owned());
            };
            parsed.link.loss_pct = parse_percent_arg(value, "--loss-pct")?;
            idx += 2;
            continue;
        }
        if arg == "--reorder-pct" {
            let Some(value) = args.get(idx + 1) else {
                return Err("missing value after --reorder-pct".to_owned());
            };
            parsed.link.reorder_pct = parse_percent_arg(value, "--reorder-pct")?;
            idx += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--bind=") {
            if value.is_empty() {
                return Err("missing value after --bind=".to_owned());
            }
            parsed.bind_addr = value.to_owned();
            idx += 1;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--latency-ms=") {
            if value.is_empty() {
                return Err("missing value after --latency-ms=".to_owned());
            }
            parsed.link.latency_ms = parse_u64_arg(value, "--latency-ms")?;
            idx += 1;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--jitter-ms=") {
            if value.is_empty() {
                return Err("missing value after --jitter-ms=".to_owned());
            }
            parsed.link.jitter_ms = parse_u64_arg(value, "--jitter-ms")?;
            idx += 1;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--loss-pct=") {
            if value.is_empty() {
                return Err("missing value after --loss-pct=".to_owned());
            }
            parsed.link.loss_pct = parse_percent_arg(value, "--loss-pct")?;
            idx += 1;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--reorder-pct=") {
            if value.is_empty() {
                return Err("missing value after --reorder-pct=".to_owned());
            }
            parsed.link.reorder_pct = parse_percent_arg(value, "--reorder-pct")?;
            idx += 1;
            continue;
        }
        return Err(format!(
            "unknown argument '{arg}'. Usage: nes-relay [--bind <addr>] [--latency-ms <n>] [--jitter-ms <n>] [--loss-pct <0..100>] [--reorder-pct <0..100>]"
        ));
    }
    Ok(parsed)
}

fn parse_u64_arg(value: &str, flag: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{flag} must be a non-negative integer"))
}

fn parse_percent_arg(value: &str, flag: &str) -> Result<u8, String> {
    let parsed = value
        .parse::<u8>()
        .map_err(|_| format!("{flag} must be an integer in [0, 100]"))?;
    if parsed > 100 {
        return Err(format!("{flag} must be in [0, 100]"));
    }
    Ok(parsed)
}

fn handle_client(
    stream: TcpStream,
    state: Arc<Mutex<RelayState>>,
    net_sim: Arc<RelayNetSim>,
) -> Result<(), String> {
    let mut reader = BufReader::new(
        stream
            .try_clone()
            .map_err(|err| format!("failed to clone stream for reader: {err}"))?,
    );
    let mut writer = stream
        .try_clone()
        .map_err(|err| format!("failed to clone stream for writer: {err}"))?;

    let (tx_out, rx_out) = mpsc::channel::<ServerMessage>();
    thread::spawn(move || {
        for message in rx_out {
            let encoded = match serde_json::to_string(&message) {
                Ok(line) => line,
                Err(err) => {
                    eprintln!("failed to serialize message: {err}");
                    break;
                }
            };
            if writer
                .write_all(encoded.as_bytes())
                .and_then(|_| writer.write_all(b"\n"))
                .and_then(|_| writer.flush())
                .is_err()
            {
                break;
            }
        }
    });

    let join = read_client_message(&mut reader)?
        .ok_or_else(|| "client disconnected before join".to_owned())?;
    let (room, player) = match join {
        ClientMessage::Join { room, player } => {
            if !matches!(player, 1 | 2) {
                let _ = tx_out.send(ServerMessage::Error {
                    message: "player must be 1 or 2".to_owned(),
                });
                return Err(format!("invalid player slot {player}"));
            }
            (room, player)
        }
        _ => {
            let _ = tx_out.send(ServerMessage::Error {
                message: "first message must be join".to_owned(),
            });
            return Err("first message must be join".to_owned());
        }
    };

    {
        let mut guard = state
            .lock()
            .map_err(|_| "relay state mutex poisoned".to_owned())?;
        let room_state = guard.rooms.entry(room.clone()).or_default();
        if room_state.players.contains_key(&player) {
            let _ = tx_out.send(ServerMessage::Error {
                message: format!("room '{room}' already has player {player}"),
            });
            return Err(format!("duplicate player slot {player} for room {room}"));
        }
        let peer_present = room_state.players.keys().any(|slot| *slot != player);
        for peer_tx in room_state.players.values() {
            let _ = peer_tx.send(ServerMessage::PeerJoined { player });
        }
        room_state.players.insert(player, tx_out.clone());
        let _ = tx_out.send(ServerMessage::Joined {
            room: room.clone(),
            player,
            peer_present,
        });
    }

    while let Some(message) = read_client_message(&mut reader)? {
        match message {
            ClientMessage::Join { .. } => {
                let _ = tx_out.send(ServerMessage::Error {
                    message: "join already completed".to_owned(),
                });
            }
            ClientMessage::Input { frame, bits } => {
                forward_to_room_peers(
                    &state,
                    &net_sim,
                    &room,
                    player,
                    ServerMessage::PeerInput {
                        player,
                        frame,
                        bits,
                    },
                )?;
            }
            ClientMessage::Hash { frame, state_hash } => {
                forward_to_room_peers(
                    &state,
                    &net_sim,
                    &room,
                    player,
                    ServerMessage::PeerHash {
                        player,
                        frame,
                        state_hash,
                    },
                )?;
            }
            ClientMessage::Ping { nonce } => {
                let _ = tx_out.send(ServerMessage::Pong { nonce });
            }
        }
    }

    cleanup_client(&state, &room, player)?;
    Ok(())
}

fn read_client_message(reader: &mut BufReader<TcpStream>) -> Result<Option<ClientMessage>, String> {
    let mut line = String::new();
    let bytes_read = reader
        .read_line(&mut line)
        .map_err(|err| format!("failed to read socket line: {err}"))?;
    if bytes_read == 0 {
        return Ok(None);
    }
    let parsed = serde_json::from_str::<ClientMessage>(line.trim())
        .map_err(|err| format!("failed to parse client message: {err}"))?;
    Ok(Some(parsed))
}

fn forward_to_room_peers(
    state: &Arc<Mutex<RelayState>>,
    net_sim: &Arc<RelayNetSim>,
    room: &str,
    from_player: u8,
    message: ServerMessage,
) -> Result<(), String> {
    let recipients = {
        let guard = state
            .lock()
            .map_err(|_| "relay state mutex poisoned".to_owned())?;
        let Some(room_state) = guard.rooms.get(room) else {
            return Ok(());
        };
        room_state
            .players
            .iter()
            .filter_map(|(slot, tx)| {
                if *slot == from_player {
                    None
                } else {
                    Some(tx.clone())
                }
            })
            .collect::<Vec<_>>()
    };

    for tx in recipients {
        if net_sim.should_drop() {
            continue;
        }
        let delayed_message = message.clone();
        let delay_ms = net_sim.sample_delay_ms();
        if delay_ms == 0 {
            let _ = tx.send(delayed_message);
            continue;
        }
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(delay_ms));
            let _ = tx.send(delayed_message);
        });
    }
    Ok(())
}

fn seed_entropy() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(1))
        .as_nanos() as u64;
    let pid = u64::from(std::process::id());
    let mixed = nanos ^ (pid.rotate_left(17));
    if mixed == 0 {
        0xD1B5_4A32_D192_ED03
    } else {
        mixed
    }
}

fn cleanup_client(state: &Arc<Mutex<RelayState>>, room: &str, player: u8) -> Result<(), String> {
    let mut guard = state
        .lock()
        .map_err(|_| "relay state mutex poisoned".to_owned())?;
    let mut should_remove_room = false;
    if let Some(room_state) = guard.rooms.get_mut(room) {
        room_state.players.remove(&player);
        for tx in room_state.players.values() {
            let _ = tx.send(ServerMessage::PeerLeft { player });
        }
        should_remove_room = room_state.players.is_empty();
    }
    if should_remove_room {
        guard.rooms.remove(room);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_BIND_ADDR, parse_args};

    #[test]
    fn parse_args_supports_fault_flags() {
        let args = vec![
            "--bind=0.0.0.0:9999".to_owned(),
            "--latency-ms=55".to_owned(),
            "--jitter-ms".to_owned(),
            "12".to_owned(),
            "--loss-pct".to_owned(),
            "7".to_owned(),
            "--reorder-pct=11".to_owned(),
        ];
        let parsed = parse_args(args).expect("parse args");
        assert_eq!(parsed.bind_addr, "0.0.0.0:9999");
        assert_eq!(parsed.link.latency_ms, 55);
        assert_eq!(parsed.link.jitter_ms, 12);
        assert_eq!(parsed.link.loss_pct, 7);
        assert_eq!(parsed.link.reorder_pct, 11);
    }

    #[test]
    fn parse_args_defaults_to_zero_faults() {
        let parsed = parse_args(Vec::new()).expect("parse default args");
        assert_eq!(parsed.bind_addr, DEFAULT_BIND_ADDR);
        assert_eq!(parsed.link.latency_ms, 0);
        assert_eq!(parsed.link.jitter_ms, 0);
        assert_eq!(parsed.link.loss_pct, 0);
        assert_eq!(parsed.link.reorder_pct, 0);
    }

    #[test]
    fn parse_args_rejects_percent_over_100() {
        let err = parse_args(vec!["--loss-pct=101".to_owned()]).expect_err("invalid percent");
        assert!(err.contains("[0, 100]"));
    }
}
