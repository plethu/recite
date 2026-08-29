use super::super::{
    DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticAuxiliaryPresentationContract,
    DiagnosticPresentationContract,
};

const NO_ARGUMENTS: &[DiagnosticArgumentSpec] = &[];
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
const INVALID_VALUE: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("function", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("index", DiagnosticArgumentType::Integer),
    DiagnosticArgumentSpec::new("expected", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("value", DiagnosticArgumentType::String),
];
const RETURN_TYPE: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("function", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("actual", DiagnosticArgumentType::String),
];
const REASON: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "reason",
    DiagnosticArgumentType::String,
)];

const UNKNOWN_FUNCTION: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE034", "diagnostic-validate-034", FUNCTION);
const WRONG_ARITY: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE035", "diagnostic-validate-035", ARITY);
const WRONG_ARGUMENT_TYPE: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE036",
    "diagnostic-validate-036",
    ARGUMENT_TYPE,
);
const INVALID_ARGUMENT_VALUE: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE037",
    "diagnostic-validate-037",
    INVALID_VALUE,
);
const WRONG_RETURN_BOOL: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE038",
    "diagnostic-validate-038-bool",
    RETURN_TYPE,
);
const WRONG_RETURN_ENUM: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE038",
    "diagnostic-validate-038-enum",
    RETURN_TYPE,
);
const UNKNOWN_REASON: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE039", "diagnostic-validate-039", REASON);
const PARAMETERIZED_REASON: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE040", "diagnostic-validate-040", REASON);
const REASON_WITHOUT_REQUIREMENT: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE041", "diagnostic-validate-041", REASON);

const UNKNOWN_FUNCTION_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-034-help", NO_ARGUMENTS);
const WRONG_ARITY_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-035-help", NO_ARGUMENTS);
const INVALID_ARGUMENT_VALUE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-037-help", NO_ARGUMENTS);
const UNKNOWN_REASON_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-039-help", NO_ARGUMENTS);
const PARAMETERIZED_REASON_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-040-help", NO_ARGUMENTS);

static CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &UNKNOWN_FUNCTION,
    &WRONG_ARITY,
    &WRONG_ARGUMENT_TYPE,
    &INVALID_ARGUMENT_VALUE,
    &WRONG_RETURN_BOOL,
    &WRONG_RETURN_ENUM,
    &UNKNOWN_REASON,
    &PARAMETERIZED_REASON,
    &REASON_WITHOUT_REQUIREMENT,
];
static AUXILIARY_CONTRACTS: &[&DiagnosticAuxiliaryPresentationContract] = &[
    &UNKNOWN_FUNCTION_HELP,
    &WRONG_ARITY_HELP,
    &INVALID_ARGUMENT_VALUE_HELP,
    &UNKNOWN_REASON_HELP,
    &PARAMETERIZED_REASON_HELP,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}

pub(super) fn auxiliary_contracts()
-> impl Iterator<Item = &'static DiagnosticAuxiliaryPresentationContract> {
    AUXILIARY_CONTRACTS.iter().copied()
}
