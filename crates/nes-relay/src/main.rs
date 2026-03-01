use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use nes_netplay::{ClientMessage, ServerMessage};

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:4545";

#[derive(Default)]
struct RelayState {
    rooms: HashMap<String, RoomState>,
}

#[derive(Default)]
struct RoomState {
    players: HashMap<u8, Sender<ServerMessage>>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let bind_addr = parse_bind_arg(std::env::args().skip(1).collect())?;
    let listener = TcpListener::bind(&bind_addr)
        .map_err(|err| format!("failed to bind {bind_addr}: {err}"))?;
    println!("nes-relay listening on {bind_addr}");

    let state = Arc::new(Mutex::new(RelayState::default()));
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
        thread::spawn(move || {
            if let Err(err) = handle_client(stream, shared) {
                eprintln!("client {peer} disconnected with error: {err}");
            }
        });
    }
    Ok(())
}

fn parse_bind_arg(args: Vec<String>) -> Result<String, String> {
    let mut bind = DEFAULT_BIND_ADDR.to_owned();
    let mut idx = 0_usize;
    while idx < args.len() {
        let arg = &args[idx];
        if arg == "--help" || arg == "-h" {
            return Err(format!(
                "Usage: nes-relay [--bind <addr>]\nDefault bind: {DEFAULT_BIND_ADDR}"
            ));
        }
        if arg == "--bind" {
            let Some(value) = args.get(idx + 1) else {
                return Err("missing value after --bind".to_owned());
            };
            bind = value.clone();
            idx += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--bind=") {
            if value.is_empty() {
                return Err("missing value after --bind=".to_owned());
            }
            bind = value.to_owned();
            idx += 1;
            continue;
        }
        return Err(format!(
            "unknown argument '{arg}'. Usage: nes-relay [--bind <addr>]"
        ));
    }
    Ok(bind)
}

fn handle_client(stream: TcpStream, state: Arc<Mutex<RelayState>>) -> Result<(), String> {
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
    room: &str,
    from_player: u8,
    message: ServerMessage,
) -> Result<(), String> {
    let guard = state
        .lock()
        .map_err(|_| "relay state mutex poisoned".to_owned())?;
    let Some(room_state) = guard.rooms.get(room) else {
        return Ok(());
    };
    for (slot, tx) in &room_state.players {
        if *slot == from_player {
            continue;
        }
        let _ = tx.send(message.clone());
    }
    Ok(())
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
