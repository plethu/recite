use crate::UiArgType;

/// A deterministic, source-independent failure in the shared UI contract.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ContractIssue {
    Malformed(String),
    Resolution(String),
    MissingId(String),
    UnknownId(String),
    DuplicateId(String),
    MissingArgument {
        id: String,
        name: String,
    },
    ExtraArgument {
        id: String,
        name: String,
    },
    SelectorArgumentMismatch {
        id: String,
        name: String,
    },
    DuplicateArgument {
        id: String,
        name: String,
    },
    ArgumentTypeMismatch {
        id: String,
        name: String,
        expected: UiArgType,
        actual: UiArgType,
    },
    DuplicateClient(String),
    UnknownClient(String),
    UndeclaredProjection {
        id: String,
        client: String,
    },
    ProjectionSourceMismatch {
        id: String,
        source: String,
    },
}

impl std::fmt::Display for ContractIssue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(detail) => write!(formatter, "malformed Fluent resource: {detail}"),
            Self::Resolution(detail) => {
                write!(formatter, "Fluent resource cannot be resolved: {detail}")
            }
            Self::MissingId(id) => write!(formatter, "missing resource ID `{id}`"),
            Self::UnknownId(id) => write!(formatter, "unknown resource ID `{id}`"),
            Self::DuplicateId(id) => write!(formatter, "duplicate resource ID `{id}`"),
            Self::MissingArgument { id, name } => {
                write!(formatter, "resource `{id}` is missing argument `{name}`")
            }
            Self::ExtraArgument { id, name } => write!(
                formatter,
                "resource `{id}` has undeclared argument `{name}`"
            ),
            Self::SelectorArgumentMismatch { id, name } => write!(
                formatter,
                "resource `{id}` uses argument `{name}` in only some selector variants"
            ),
            Self::DuplicateArgument { id, name } => write!(
                formatter,
                "resource `{id}` declares duplicate argument `{name}`"
            ),
            Self::ArgumentTypeMismatch {
                id,
                name,
                expected,
                actual,
            } => write!(
                formatter,
                "resource `{id}` argument `{name}` has type {actual:?}, expected {expected:?}"
            ),
            Self::DuplicateClient(client) => write!(formatter, "duplicate client `{client}`"),
            Self::UnknownClient(client) => write!(formatter, "unknown client `{client}`"),
            Self::UndeclaredProjection { id, client } => write!(
                formatter,
                "resource `{id}` has undeclared projection for `{client}`"
            ),
            Self::ProjectionSourceMismatch { id, source } => write!(
                formatter,
                "projection for resource `{id}` records unknown source resource `{source}`"
            ),
        }
    }
}

/// All failures from one contract validation, sorted deterministically.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiContractError {
    pub issues: Vec<ContractIssue>,
}

impl std::fmt::Display for UiContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, issue) in self.issues.iter().enumerate() {
            if index > 0 {
                formatter.write_str("; ")?;
            }
            issue.fmt(formatter)?;
        }
        Ok(())
    }
}

impl std::error::Error for UiContractError {}
