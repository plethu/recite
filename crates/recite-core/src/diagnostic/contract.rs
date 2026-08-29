use super::DiagnosticCode;
use crate::diagnostic_argument::DiagnosticArgumentValue;
use crate::diagnostic_presentation::DiagnosticPresentationId;
use crate::diagnostic_presentation_record::DiagnosticPresentation;

mod compiler;
mod config;
mod freshness;
mod parser;
mod po;
mod project;
mod registry;
mod schema;

pub use registry::{
    DiagnosticPresentationContractRegistryError, auxiliary_contract_for, contract_for,
    contracts_for_code, migrated_diagnostic_auxiliary_presentation_contracts,
    migrated_diagnostic_presentation_contracts, presentation_for,
    validate_auxiliary_diagnostic_presentation_contracts,
    validate_diagnostic_presentation_contracts,
    validate_migrated_diagnostic_presentation_contracts,
};

/// The format-neutral type of one named diagnostic presentation argument.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum DiagnosticArgumentType {
    String,
    Integer,
    Float,
    Boolean,
}

impl DiagnosticArgumentType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Boolean => "boolean",
        }
    }
}

/// A named argument in a producer-owned diagnostic presentation contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct DiagnosticArgumentSpec {
    name: &'static str,
    argument_type: DiagnosticArgumentType,
}

impl DiagnosticArgumentSpec {
    #[must_use]
    pub const fn new(name: &'static str, argument_type: DiagnosticArgumentType) -> Self {
        Self {
            name,
            argument_type,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    #[must_use]
    pub const fn argument_type(self) -> DiagnosticArgumentType {
        self.argument_type
    }
}

/// The central contract between one diagnostic producer and its presentation
/// resource. Codes remain machine-facing identity; the presentation ID is the
/// locale-neutral resource identity used at client boundaries.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DiagnosticPresentationContract {
    code: DiagnosticCode,
    presentation_id: DiagnosticPresentationId,
    arguments: &'static [DiagnosticArgumentSpec],
}

/// A structured presentation used for diagnostic related spans or help.
///
/// Auxiliary presentations intentionally do not carry a diagnostic code:
/// they are attached to a primary producer record and are not themselves
/// diagnostic identity. Keeping this contract separate prevents a related
/// message from accidentally being treated as a second primary diagnostic.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DiagnosticAuxiliaryPresentationContract {
    presentation_id: DiagnosticPresentationId,
    arguments: &'static [DiagnosticArgumentSpec],
}

impl DiagnosticAuxiliaryPresentationContract {
    #[must_use]
    pub const fn new(
        presentation_id: &'static str,
        arguments: &'static [DiagnosticArgumentSpec],
    ) -> Self {
        Self {
            presentation_id: DiagnosticPresentationId::new_static(presentation_id),
            arguments,
        }
    }

    #[must_use]
    pub fn presentation_id(&self) -> &DiagnosticPresentationId {
        &self.presentation_id
    }

    #[must_use]
    pub const fn arguments(&self) -> &'static [DiagnosticArgumentSpec] {
        self.arguments
    }

    /// Build an auxiliary presentation while enforcing its exact signature.
    pub fn presentation<I, K>(
        &self,
        arguments: I,
    ) -> Result<
        crate::diagnostic_presentation_record::DiagnosticPresentation,
        crate::DiagnosticPresentationError,
    >
    where
        I: IntoIterator<Item = (K, DiagnosticArgumentValue)>,
        K: Into<String>,
    {
        let presentation =
            crate::diagnostic_presentation_record::DiagnosticPresentation::from_arguments(
                self.presentation_id.clone(),
                arguments,
            )?;
        for argument in self.arguments() {
            let Some(value) = presentation.arguments().get(argument.name()) else {
                return Err(crate::DiagnosticPresentationError::MissingArgument(
                    argument.name().to_owned(),
                ));
            };
            if value.argument_type() != argument.argument_type() {
                return Err(crate::DiagnosticPresentationError::ArgumentTypeMismatch {
                    name: argument.name().to_owned(),
                    expected: argument.argument_type(),
                    actual: value.argument_type(),
                });
            }
        }
        for name in presentation.arguments().keys() {
            if !self
                .arguments()
                .iter()
                .any(|argument| argument.name() == name)
            {
                return Err(crate::DiagnosticPresentationError::ExtraArgument(
                    name.clone(),
                ));
            }
        }
        Ok(presentation)
    }
}

impl DiagnosticPresentationContract {
    #[must_use]
    pub const fn new(
        code: &'static str,
        presentation_id: &'static str,
        arguments: &'static [DiagnosticArgumentSpec],
    ) -> Self {
        Self {
            code: DiagnosticCode::new_static(code),
            presentation_id: DiagnosticPresentationId::new_static(presentation_id),
            arguments,
        }
    }

    #[must_use]
    pub fn code(&self) -> &DiagnosticCode {
        &self.code
    }

    #[must_use]
    pub fn presentation_id(&self) -> &DiagnosticPresentationId {
        &self.presentation_id
    }

    #[must_use]
    pub const fn arguments(&self) -> &'static [DiagnosticArgumentSpec] {
        self.arguments
    }

    /// Build the locale-neutral presentation while enforcing this producer's
    /// exact named argument contract.
    pub fn presentation<I, K>(
        &self,
        arguments: I,
    ) -> Result<DiagnosticPresentation, crate::DiagnosticPresentationError>
    where
        I: IntoIterator<Item = (K, DiagnosticArgumentValue)>,
        K: Into<String>,
    {
        DiagnosticPresentation::from_contract(self, arguments)
    }
}
