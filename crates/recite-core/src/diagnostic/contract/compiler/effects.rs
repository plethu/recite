use super::super::{
    DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticAuxiliaryPresentationContract,
    DiagnosticPresentationContract,
};

const FUNCTION: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "function",
    DiagnosticArgumentType::String,
)];
const ARITY: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("function", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("expected", DiagnosticArgumentType::Integer),
    DiagnosticArgumentSpec::new("actual", DiagnosticArgumentType::Integer),
];
const ARGUMENT_TYPE: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("function", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("index", DiagnosticArgumentType::Integer),
    DiagnosticArgumentSpec::new("expected", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("actual", DiagnosticArgumentType::String),
];
const MODE: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("function", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("mode", DiagnosticArgumentType::String),
];
const INVALID_VALUE: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("function", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("index", DiagnosticArgumentType::Integer),
    DiagnosticArgumentSpec::new("expected", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("value", DiagnosticArgumentType::String),
];

const UNKNOWN_FUNCTION: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE017", "diagnostic-validate-017", FUNCTION);
const WRONG_ARITY: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE018", "diagnostic-validate-018", ARITY);
const WRONG_ARGUMENT_TYPE: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE019",
    "diagnostic-validate-019",
    ARGUMENT_TYPE,
);
const UNSUPPORTED_MODE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE020", "diagnostic-validate-020", MODE);
const INVALID_ARGUMENT_VALUE: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE021",
    "diagnostic-validate-021",
    INVALID_VALUE,
);

const UNKNOWN_FUNCTION_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-017-help", &[]);
const WRONG_ARITY_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-018-help", &[]);
const UNSUPPORTED_MODE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-020-help", &[]);
const INVALID_ARGUMENT_VALUE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-021-help", &[]);

static CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &UNKNOWN_FUNCTION,
    &WRONG_ARITY,
    &WRONG_ARGUMENT_TYPE,
    &UNSUPPORTED_MODE,
    &INVALID_ARGUMENT_VALUE,
];
static AUXILIARY_CONTRACTS: &[&DiagnosticAuxiliaryPresentationContract] = &[
    &UNKNOWN_FUNCTION_HELP,
    &WRONG_ARITY_HELP,
    &UNSUPPORTED_MODE_HELP,
    &INVALID_ARGUMENT_VALUE_HELP,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}

pub(super) fn auxiliary_contracts()
-> impl Iterator<Item = &'static DiagnosticAuxiliaryPresentationContract> {
    AUXILIARY_CONTRACTS.iter().copied()
}
