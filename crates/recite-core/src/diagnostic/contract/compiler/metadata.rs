use super::super::{
    DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticAuxiliaryPresentationContract,
    DiagnosticPresentationContract,
};

const KEY: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "key",
    DiagnosticArgumentType::String,
)];
const KEY_TARGET: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("key", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("target", DiagnosticArgumentType::String),
];
const KEY_TYPES: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("key", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("expected", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("actual", DiagnosticArgumentType::String),
];
const KEY_TYPE_VALUE: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("key", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("expected", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("value", DiagnosticArgumentType::String),
];
const KEY_DOMAIN_VALUE: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("key", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("domain", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("value", DiagnosticArgumentType::String),
];
const KEY_DOMAIN_SELECTOR: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("key", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("domain", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("selector", DiagnosticArgumentType::String),
];
const KEY_SELECTOR: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("key", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("selector", DiagnosticArgumentType::String),
];

const UNKNOWN_KEY: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE026", "diagnostic-validate-026", KEY);
const INVALID_TARGET: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE027",
    "diagnostic-validate-027",
    KEY_TARGET,
);
const DUPLICATE_KEY: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE028", "diagnostic-validate-028", KEY);
const WRONG_VALUE_TYPE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE029", "diagnostic-validate-029", KEY_TYPES);
const INVALID_VALUE: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE030",
    "diagnostic-validate-030",
    KEY_TYPE_VALUE,
);
const INVALID_DOMAIN_VALUE: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE031",
    "diagnostic-validate-031",
    KEY_DOMAIN_VALUE,
);
const MISSING_DOMAIN_CONTEXT: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE032",
    "diagnostic-validate-032",
    KEY_DOMAIN_SELECTOR,
);
const MALFORMED_DOMAIN_CONTEXT: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_VALIDATE033",
        "diagnostic-validate-033",
        KEY_SELECTOR,
    );

const UNKNOWN_KEY_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-026-help", &[]);
const INVALID_TARGET_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-027-help", &[]);
const DUPLICATE_KEY_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-028-help", &[]);
const WRONG_VALUE_TYPE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-029-help", &[]);
const INVALID_VALUE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-030-help", &[]);
const INVALID_DOMAIN_VALUE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-031-help", &[]);
const MISSING_DOMAIN_CONTEXT_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-032-help", &[]);
const MALFORMED_DOMAIN_CONTEXT_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-033-help", &[]);

static CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &UNKNOWN_KEY,
    &INVALID_TARGET,
    &DUPLICATE_KEY,
    &WRONG_VALUE_TYPE,
    &INVALID_VALUE,
    &INVALID_DOMAIN_VALUE,
    &MISSING_DOMAIN_CONTEXT,
    &MALFORMED_DOMAIN_CONTEXT,
];
static AUXILIARY_CONTRACTS: &[&DiagnosticAuxiliaryPresentationContract] = &[
    &UNKNOWN_KEY_HELP,
    &INVALID_TARGET_HELP,
    &DUPLICATE_KEY_HELP,
    &WRONG_VALUE_TYPE_HELP,
    &INVALID_VALUE_HELP,
    &INVALID_DOMAIN_VALUE_HELP,
    &MISSING_DOMAIN_CONTEXT_HELP,
    &MALFORMED_DOMAIN_CONTEXT_HELP,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}

pub(super) fn auxiliary_contracts()
-> impl Iterator<Item = &'static DiagnosticAuxiliaryPresentationContract> {
    AUXILIARY_CONTRACTS.iter().copied()
}
