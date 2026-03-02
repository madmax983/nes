use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Join { room: String, player: u8 },
    Input { frame: u64, bits: u8 },
    Hash { frame: u64, state_hash: u64 },
    Ping { nonce: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Joined {
        room: String,
        player: u8,
        peer_present: bool,
    },
    PeerJoined {
        player: u8,
    },
    PeerLeft {
        player: u8,
    },
    PeerInput {
        player: u8,
        frame: u64,
        bits: u8,
    },
    PeerHash {
        player: u8,
        frame: u64,
        state_hash: u64,
    },
    Pong {
        nonce: u64,
    },
    Error {
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
