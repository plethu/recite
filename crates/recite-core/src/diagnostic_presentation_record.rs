use std::collections::BTreeMap;
use std::fmt;

use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::diagnostic::DiagnosticPresentationContract;
use super::diagnostic_argument::DiagnosticArgumentValue;
use super::diagnostic_presentation::{DiagnosticPresentationError, DiagnosticPresentationId};

/// Deterministically ordered named arguments for a diagnostic presentation.
pub type DiagnosticArguments = BTreeMap<String, DiagnosticArgumentValue>;

/// A locale-neutral resource reference and deterministic named arguments.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DiagnosticPresentation {
    id: DiagnosticPresentationId,
    arguments: DiagnosticArguments,
}

impl DiagnosticPresentation {
    #[must_use]
    pub fn new(id: DiagnosticPresentationId) -> Self {
        Self {
            id,
            arguments: BTreeMap::new(),
        }
    }

    /// Construct a presentation from named arguments after validating all keys
    /// and values.
    pub fn from_arguments<I, K>(
        id: DiagnosticPresentationId,
        arguments: I,
    ) -> Result<Self, DiagnosticPresentationError>
    where
        I: IntoIterator<Item = (K, DiagnosticArgumentValue)>,
        K: Into<String>,
    {
        let mut presentation = Self::new(id);
        for (name, value) in arguments {
            presentation.insert_argument(name, value)?;
        }
        Ok(presentation)
    }

    /// Construct a presentation while enforcing the exact argument contract
    /// declared by its producer.
    pub fn from_contract<I, K>(
        contract: &DiagnosticPresentationContract,
        arguments: I,
    ) -> Result<Self, DiagnosticPresentationError>
    where
        I: IntoIterator<Item = (K, DiagnosticArgumentValue)>,
        K: Into<String>,
    {
        let presentation = Self::from_arguments(contract.presentation_id().clone(), arguments)?;
        for argument in contract.arguments() {
            let Some(value) = presentation.arguments.get(argument.name()) else {
                return Err(DiagnosticPresentationError::MissingArgument(
                    argument.name().to_owned(),
                ));
            };
            if value.argument_type() != argument.argument_type() {
                return Err(DiagnosticPresentationError::ArgumentTypeMismatch {
                    name: argument.name().to_owned(),
                    expected: argument.argument_type(),
                    actual: value.argument_type(),
                });
            }
        }
        for name in presentation.arguments.keys() {
            if !contract
                .arguments()
                .iter()
                .any(|argument| argument.name() == name)
            {
                return Err(DiagnosticPresentationError::ExtraArgument(name.clone()));
            }
        }
        Ok(presentation)
    }

    #[must_use]
    pub fn id(&self) -> &DiagnosticPresentationId {
        &self.id
    }

    /// Arguments are ordered by name, making serialization and rendering
    /// deterministic regardless of producer insertion order.
    #[must_use]
    pub fn arguments(&self) -> &DiagnosticArguments {
        &self.arguments
    }

    pub fn with_argument(
        mut self,
        name: impl Into<String>,
        value: DiagnosticArgumentValue,
    ) -> Result<Self, DiagnosticPresentationError> {
        self.insert_argument(name, value)?;
        Ok(self)
    }

    pub fn insert_argument(
        &mut self,
        name: impl Into<String>,
        value: DiagnosticArgumentValue,
    ) -> Result<(), DiagnosticPresentationError> {
        let name = name.into();
        if !is_valid_argument_name(&name) {
            return Err(DiagnosticPresentationError::InvalidArgumentName(name));
        }
        value.validate()?;
        if self.arguments.contains_key(&name) {
            return Err(DiagnosticPresentationError::DuplicateArgument(name));
        }
        self.arguments.insert(name, value);
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticPresentationWire {
    id: DiagnosticPresentationId,
    #[serde(deserialize_with = "deserialize_arguments")]
    arguments: DiagnosticArguments,
}

fn deserialize_arguments<'de, D>(deserializer: D) -> Result<DiagnosticArguments, D::Error>
where
    D: Deserializer<'de>,
{
    struct ArgumentsVisitor;

    impl<'de> Visitor<'de> for ArgumentsVisitor {
        type Value = DiagnosticArguments;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a map of unique diagnostic argument names")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut arguments = DiagnosticArguments::new();
            while let Some((name, value)) = map.next_entry::<String, DiagnosticArgumentValue>()? {
                if arguments.insert(name.clone(), value).is_some() {
                    return Err(serde::de::Error::custom(format_args!(
                        "duplicate diagnostic argument `{name}`"
                    )));
                }
            }
            Ok(arguments)
        }
    }

    deserializer.deserialize_map(ArgumentsVisitor)
}

impl Serialize for DiagnosticPresentation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        DiagnosticPresentationWire {
            id: self.id.clone(),
            arguments: self.arguments.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DiagnosticPresentation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DiagnosticPresentationWire::deserialize(deserializer)?;
        Self::from_arguments(wire.id, wire.arguments).map_err(serde::de::Error::custom)
    }
}

pub(crate) fn is_valid_argument_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && is_lowercase_letter(bytes[0])
        && bytes
            .iter()
            .copied()
            .all(|byte| is_lowercase_letter(byte) || is_digit(byte) || byte == b'_')
}

const fn is_lowercase_letter(byte: u8) -> bool {
    byte.is_ascii_lowercase()
}

const fn is_digit(byte: u8) -> bool {
    byte.is_ascii_digit()
}
