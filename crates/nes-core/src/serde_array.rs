use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn serialize_u8_array<S, const N: usize>(
    value: &[u8; N],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value.as_slice().serialize(serializer)
}

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
