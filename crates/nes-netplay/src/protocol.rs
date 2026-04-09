//! The core network protocol for `nes-netplay`.
//!
//! This module defines the JSON-based message structure used for communication between the emulator clients
//! and the `nes-relay` server. It enables matchmaking and deterministic input synchronization.

use serde::{Deserialize, Serialize};

/// A message sent from a client to the `nes-relay` server.
///
/// The `ClientMessage` forms the vocabulary that allows an individual user's emulator instance to speak
/// directly to the central coordination hub. It covers the complete lifecycle: from the initial handshake
/// to join a room, through the relentless synchronized pulse of joypad inputs and checksums, and finally
/// the heartbeat pings ensuring the connection remains steadfast.
///
/// ## Examples
///
/// ```rust
/// use nes_netplay::ClientMessage;
/// use serde_json;
///
/// let join_msg = ClientMessage::Join { room: "Zelda123".to_string(), player: 1 };
/// let json_str = serde_json::to_string(&join_msg).unwrap();
///
/// assert_eq!(json_str, r#"{"type":"join","room":"Zelda123","player":1}"#);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Request to join a specific matchmaking room as a specific player.
    Join {
        /// The room identifier.
        room: String,
        /// The player slot (1 or 2).
        player: u8,
    },
    /// Synchronized joypad input for a specific frame.
    Input {
        /// The frame number this input applies to.
        frame: u64,
        /// The 8-bit joypad state.
        bits: u8,
    },
    /// Synchronization hash to detect desyncs.
    Hash {
        /// The frame number of the hash.
        frame: u64,
        /// The generated state hash.
        state_hash: u64,
    },
    /// Keep-alive ping to measure latency and maintain connection.
    Ping {
        /// A unique nonce to match with the corresponding Pong.
        nonce: u64,
    },
}

/// A message sent from the `nes-relay` server to a client.
///
/// The `ServerMessage` acts as the voice of the central authority. It broadcasts the arrival and departure
/// of peers, relays crucial inputs required to keep the simulation deterministic, and serves as the final
/// arbiter when network conditions sour or validation fails.
///
/// ## Examples
///
/// ```rust
/// use nes_netplay::ServerMessage;
/// use serde_json;
///
/// let input_msg = ServerMessage::PeerInput { player: 2, frame: 1045, bits: 0b1000_0000 };
/// let json_str = serde_json::to_string(&input_msg).unwrap();
///
/// assert_eq!(json_str, r#"{"type":"peer_input","player":2,"frame":1045,"bits":128}"#);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Acknowledgment that the client successfully joined the room.
    Joined {
        /// The room identifier.
        room: String,
        /// The assigned player slot.
        player: u8,
        /// Whether the peer is already in the room.
        peer_present: bool,
    },
    /// Notification that the peer has joined the room.
    PeerJoined {
        /// The peer's player slot.
        player: u8,
    },
    /// Notification that the peer has disconnected from the room.
    PeerLeft {
        /// The peer's player slot.
        player: u8,
    },
    /// Relayed joypad input from the peer.
    PeerInput {
        /// The peer's player slot.
        player: u8,
        /// The frame number this input applies to.
        frame: u64,
        /// The 8-bit joypad state.
        bits: u8,
    },
    /// Relayed synchronization hash from the peer.
    PeerHash {
        /// The peer's player slot.
        player: u8,
        /// The frame number of the hash.
        frame: u64,
        /// The generated state hash.
        state_hash: u64,
    },
    /// Response to a [`ClientMessage::Ping`].
    Pong {
        /// The unique nonce matching the initial Ping.
        nonce: u64,
    },
    /// A fatal protocol error or connection rejection.
    Error {
        /// The error description.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::{ClientMessage, ServerMessage};

    #[test]
    fn protocol_messages_round_trip_json() {
        let client = ClientMessage::Input {
            frame: 42,
            bits: 0x83,
        };
        let encoded_client = serde_json::to_string(&client).expect("serialize");
        let decoded_client: ClientMessage =
            serde_json::from_str(&encoded_client).expect("deserialize");
        assert_eq!(decoded_client, client);

        let server = ServerMessage::PeerInput {
            player: 2,
            frame: 42,
            bits: 0x40,
        };
        let encoded_server = serde_json::to_string(&server).expect("serialize");
        let decoded_server: ServerMessage =
            serde_json::from_str(&encoded_server).expect("deserialize");
        assert_eq!(decoded_server, server);
    }
}
