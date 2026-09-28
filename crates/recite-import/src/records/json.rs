use serde::{
    Deserialize, Deserializer,
    de::{Error, MapAccess, Visitor},
};
use serde_json::Value;

// Preserve field order and reject duplicate keys before JSON can overwrite
// evidence. Nested values are never mapped and remain explicitly unsupported.
pub(super) struct UniqueRecord(pub(super) Vec<(String, Value)>);

impl<'de> Deserialize<'de> for UniqueRecord {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct RecordVisitor;
        impl<'de> Visitor<'de> for RecordVisitor {
            type Value = UniqueRecord;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a flat object with distinct field names")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut fields = Vec::<(String, Value)>::new();
                while let Some((name, value)) = map.next_entry::<String, Value>()? {
                    if fields.iter().any(|(existing, _)| existing == &name) {
                        return Err(M::Error::custom(format!("duplicate field: {name}")));
                    }
                    fields.push((name, value));
                }
                Ok(UniqueRecord(fields))
            }
        }
        deserializer.deserialize_map(RecordVisitor)
    }
}
