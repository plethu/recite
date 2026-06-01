use std::fmt;

use crate::CoreValueError;

#[cfg(feature = "small-ids")]
use compact_str::CompactString as IdStorage;

#[cfg(not(feature = "small-ids"))]
type IdStorage = String;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
        pub struct $name(IdStorage);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, CoreValueError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(CoreValueError::EmptyId {
                        kind: stringify!($name),
                    });
                }

                Ok(Self(IdStorage::from(value)))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl TryFrom<&str> for $name {
            type Error = CoreValueError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = CoreValueError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
    };
}

define_id!(LineId);
define_id!(ChoiceId);
define_id!(BlockId);
define_id!(EffectId);
define_id!(LocaleId);
define_id!(SpeakerId);
