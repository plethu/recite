use std::collections::BTreeMap;

use serde::{Deserialize, de::Error as _};
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawProducerIdentity {
    pub(crate) kind: String,
    pub(crate) id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawContentFingerprint {
    pub(crate) algorithm: String,
    pub(crate) value: String,
}

#[derive(Debug)]
pub(crate) struct RawProducerOrigin {
    pub(crate) kind: String,
    pub(crate) id: String,
    pub(crate) label: Option<String>,
    pub(crate) extensions: BTreeMap<String, Value>,
}

impl<'de> Deserialize<'de> for RawProducerOrigin {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if value.is_string() {
            return Err(D::Error::custom(
                "legacy string producer origins were removed in the pre-1.0 v1 reset; use an object with kind and id",
            ));
        }
        #[derive(Deserialize)]
        struct Origin {
            kind: String,
            id: String,
            #[serde(
                default,
                deserialize_with = "crate::schema::manifest::raw::deserialize_optional_non_null"
            )]
            label: Option<String>,
            #[serde(flatten)]
            extensions: BTreeMap<String, Value>,
        }
        let origin = serde_json::from_value::<Origin>(value).map_err(D::Error::custom)?;
        Ok(Self {
            kind: origin.kind,
            id: origin.id,
            label: origin.label,
            extensions: origin.extensions,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawProducerFingerprint {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) algorithm: String,
    pub(crate) value: String,
}
