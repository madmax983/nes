use nes_netplay::{ClientMessage, ServerMessage};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_netplay_client_deserialize(json in ".*") {
        let _ = serde_json::from_str::<ClientMessage>(&json);
    }

    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_netplay_server_deserialize(json in ".*") {
        let _ = serde_json::from_str::<ServerMessage>(&json);
    }
}
