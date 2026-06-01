use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;

pub fn serialize_arc_u8_slice<S>(value: &Arc<[u8]>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value.as_ref().serialize(serializer)
}

pub fn deserialize_arc_u8_slice<'de, D>(deserializer: D) -> Result<Arc<[u8]>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = Vec::<u8>::deserialize(deserializer)?;
    Ok(values.into())
}
