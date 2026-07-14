use serde::{Deserialize, Serialize};
use nes_core::serde_array::{serialize_u8_array, deserialize_u8_array};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct TestStruct {
    #[serde(
        serialize_with = "serialize_u8_array",
        deserialize_with = "deserialize_u8_array"
    )]
    data: [u8; 4],
}

#[test]
fn test_serialize_deserialize_u8_array_strict() {
    let test_obj = TestStruct { data: [5, 6, 7, 8] };
    let serialized = serde_json::to_string(&test_obj).unwrap();
    assert_eq!(serialized, r#"{"data":[5,6,7,8]}"#);

    let deserialized: TestStruct = serde_json::from_str(&serialized).unwrap();
    assert_eq!(test_obj, deserialized);
}
