use std::fmt;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{EffectWire, PromptWire};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WaitingForChoiceWire {
    pub(crate) prompt: Box<PromptWire>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WaitingForEffectWire {
    pub(crate) effect: EffectWire,
}

#[derive(Clone, Debug)]
pub(crate) enum StatusWire {
    Ready,
    WaitingForChoice(WaitingForChoiceWire),
    WaitingForEffect(WaitingForEffectWire),
    Ended,
}

impl Serialize for StatusWire {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Ready => serializer.serialize_unit_variant("StatusWire", 0, "Ready"),
            Self::WaitingForChoice(payload) => {
                serializer.serialize_newtype_variant("StatusWire", 1, "WaitingForChoice", payload)
            }
            Self::WaitingForEffect(payload) => {
                serializer.serialize_newtype_variant("StatusWire", 2, "WaitingForEffect", payload)
            }
            Self::Ended => serializer.serialize_unit_variant("StatusWire", 3, "Ended"),
        }
    }
}

impl<'de> Deserialize<'de> for StatusWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StatusVisitor)
    }
}

struct StatusVisitor;

impl<'de> Visitor<'de> for StatusVisitor {
    type Value = StatusWire;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a preview status string or externally tagged status map")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "Ready" => Ok(StatusWire::Ready),
            "Ended" => Ok(StatusWire::Ended),
            _ => Err(de::Error::unknown_variant(value, VARIANTS)),
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(&value)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let Some(variant) = map.next_key::<String>()? else {
            return Err(de::Error::custom("preview status map is empty"));
        };
        let status = match variant.as_str() {
            "WaitingForChoice" => {
                StatusWire::WaitingForChoice(map.next_value::<WaitingForChoiceWire>()?)
            }
            "WaitingForEffect" => {
                StatusWire::WaitingForEffect(map.next_value::<WaitingForEffectWire>()?)
            }
            "Ready" | "Ended" => {
                return Err(de::Error::custom(
                    "unit preview status must be encoded as a string",
                ));
            }
            _ => return Err(de::Error::unknown_variant(&variant, VARIANTS)),
        };
        if let Some(extra) = map.next_key::<String>()? {
            return Err(de::Error::unknown_field(&extra, &[]));
        }
        Ok(status)
    }
}

const VARIANTS: &[&str] = &["Ready", "WaitingForChoice", "WaitingForEffect", "Ended"];
