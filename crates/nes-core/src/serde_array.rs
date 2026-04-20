//! Provides custom Serde serialization and deserialization for fixed-size byte arrays `[u8; N]`.
//!
//! This module addresses Serde's default behavior where fixed-size arrays larger than 32 elements
//! might not be supported cleanly by default, or where specific representation constraints are needed.
//! By using `#[serde(with = "serde_array")]`, users can consistently serialize byte buffers
//! across JSON, CBOR, and other formats.

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Serializes a fixed-size byte array `[u8; N]`.
///
/// This function is intended to be used with Serde's `#[serde(serialize_with = "...")]` field attribute.
///
/// ## Examples
///
/// ```
/// use serde::Serialize;
/// use nes_core::serde_array::serialize_u8_array;
///
/// #[derive(Serialize)]
/// struct Packet {
///     #[serde(serialize_with = "serialize_u8_array")]
///     data: [u8; 4],
/// }
///
/// let packet = Packet { data: [1, 2, 3, 4] };
/// let json = serde_json::to_string(&packet).unwrap();
/// assert_eq!(json, r#"{"data":[1,2,3,4]}"#);
/// ```
pub fn serialize_u8_array<S, const N: usize>(
    value: &[u8; N],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value.as_slice().serialize(serializer)
}

/// Deserializes a fixed-size byte array `[u8; N]`.
///
/// This function is intended to be used with Serde's `#[serde(deserialize_with = "...")]` field attribute.
///
/// ## Examples
///
/// ```
/// use serde::Deserialize;
/// use nes_core::serde_array::deserialize_u8_array;
///
/// #[derive(Deserialize, Debug, PartialEq)]
/// struct Packet {
///     #[serde(deserialize_with = "deserialize_u8_array")]
///     data: [u8; 4],
/// }
///
/// let json = r#"{"data":[1,2,3,4]}"#;
/// let packet: Packet = serde_json::from_str(json).unwrap();
/// assert_eq!(packet.data, [1, 2, 3, 4]);
/// ```
pub fn deserialize_u8_array<'de, D, const N: usize>(deserializer: D) -> Result<[u8; N], D::Error>
where
    D: Deserializer<'de>,
{
    let values = Vec::<u8>::deserialize(deserializer)?;
    values
        .try_into()
        .map_err(|values: Vec<u8>| D::Error::invalid_length(values.len(), &ExpectedLength(N)))
}

struct ExpectedLength(usize);

impl serde::de::Expected for ExpectedLength {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a byte array of length {}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestStruct {
        #[serde(
            serialize_with = "serialize_u8_array",
            deserialize_with = "deserialize_u8_array"
        )]
        data: [u8; 4],
    }

    #[test]
    fn test_serialize_deserialize_u8_array() {
        let test_obj = TestStruct { data: [1, 2, 3, 4] };
        let serialized = serde_json::to_string(&test_obj).unwrap();
        assert_eq!(serialized, r#"{"data":[1,2,3,4]}"#);

        let deserialized: TestStruct = serde_json::from_str(&serialized).unwrap();
        assert_eq!(test_obj, deserialized);
    }

    #[test]
    fn test_deserialize_invalid_length() {
        let invalid_json = r#"{"data":[1,2,3]}"#;
        let result: Result<TestStruct, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("invalid length 3, expected a byte array of length 4"));
    }

    #[test]
    fn test_expected_length_formatting() {
        let expected = ExpectedLength(42);

        let err: serde_json::Error = serde::de::Error::invalid_length(99, &expected);
        assert_eq!(
            err.to_string(),
            "invalid length 99, expected a byte array of length 42"
        );
    }
}
