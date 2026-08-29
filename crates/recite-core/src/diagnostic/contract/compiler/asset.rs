use super::super::{
    DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticAuxiliaryPresentationContract,
    DiagnosticPresentationContract,
};

const NO_ARGUMENTS: &[DiagnosticArgumentSpec] = &[];
const KEY: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "key",
    DiagnosticArgumentType::String,
)];
const LINE_CHILD: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("line_id", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("statement_kind", DiagnosticArgumentType::String),
];
const CHOICE_CHILD: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("choice_id", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("statement_kind", DiagnosticArgumentType::String),
];
const LINE_ID: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "line_id",
    DiagnosticArgumentType::String,
)];
const INVALID_SPAN_OWNER: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "owner",
    DiagnosticArgumentType::String,
)];

const INVALID_SOURCE_SPAN_FILE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_VALIDATE008",
        "diagnostic-validate-008-file",
        INVALID_SPAN_OWNER,
    );
const INVALID_SOURCE_SPAN_ORDER: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_VALIDATE008",
        "diagnostic-validate-008-order",
        INVALID_SPAN_OWNER,
    );
const MISSING_CHOICE_TARGET: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE012",
    "diagnostic-validate-012",
    NO_ARGUMENTS,
);
const UNSUPPORTED_LINE_CHILD: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE013",
    "diagnostic-validate-013",
    LINE_CHILD,
);
const UNSUPPORTED_CHOICE_CHILD: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_VALIDATE014",
        "diagnostic-validate-014",
        CHOICE_CHILD,
    );
const UNKNOWN_CHOICE_ECHO: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE015", "diagnostic-validate-015", LINE_ID);
const NON_FINITE_FLOAT: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE016",
    "diagnostic-validate-016-condition",
    NO_ARGUMENTS,
);
const NON_FINITE_EFFECT_FLOAT: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE016",
    "diagnostic-validate-016-effect",
    NO_ARGUMENTS,
);
const NON_FINITE_METADATA_FLOAT: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_VALIDATE016",
        "diagnostic-validate-016-metadata",
        KEY,
    );
const INVALID_INTERPOLATION_SYNTAX: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_VALIDATE045",
        "diagnostic-validate-045-unterminated",
        NO_ARGUMENTS,
    );
const INVALID_INTERPOLATION_UNESCAPED: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_VALIDATE045",
        "diagnostic-validate-045-unescaped",
        NO_ARGUMENTS,
    );
const INVALID_INTERPOLATION_NAME: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_VALIDATE045",
        "diagnostic-validate-045-invalid-name",
        KEY,
    );
const INVALID_INTERPOLATION_DUPLICATE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_VALIDATE045",
        "diagnostic-validate-045-duplicate",
        KEY,
    );
const INVALID_INTERPOLATION_UNUSED: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_VALIDATE045",
        "diagnostic-validate-045-unused",
        KEY,
    );
const INVALID_INTERPOLATION_UNBOUND: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_VALIDATE045",
        "diagnostic-validate-045-unbound",
        KEY,
    );
const INVALID_PLURAL_NEWLINE: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE046",
    "diagnostic-validate-046-newline",
    NO_ARGUMENTS,
);
const INVALID_PLURAL_MISSING_COUNT: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_VALIDATE046",
        "diagnostic-validate-046-missing-count",
        NO_ARGUMENTS,
    );
const INVALID_PLURAL_COUNT_TYPE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_VALIDATE046",
        "diagnostic-validate-046-count-type",
        NO_ARGUMENTS,
    );

const MISSING_CHOICE_TARGET_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-012-help", NO_ARGUMENTS);
const UNSUPPORTED_LINE_CHILD_RELATED: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-013-related", NO_ARGUMENTS);
const UNSUPPORTED_LINE_CHILD_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-013-help", NO_ARGUMENTS);
const UNSUPPORTED_CHOICE_CHILD_RELATED: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-014-related", NO_ARGUMENTS);
const UNSUPPORTED_CHOICE_CHILD_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-014-help", NO_ARGUMENTS);
const UNKNOWN_CHOICE_ECHO_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-015-help", NO_ARGUMENTS);
const NON_FINITE_FLOAT_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-016-help", NO_ARGUMENTS);

static CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &INVALID_SOURCE_SPAN_FILE,
    &INVALID_SOURCE_SPAN_ORDER,
    &MISSING_CHOICE_TARGET,
    &UNSUPPORTED_LINE_CHILD,
    &UNSUPPORTED_CHOICE_CHILD,
    &UNKNOWN_CHOICE_ECHO,
    &NON_FINITE_FLOAT,
    &NON_FINITE_EFFECT_FLOAT,
    &NON_FINITE_METADATA_FLOAT,
    &INVALID_INTERPOLATION_SYNTAX,
    &INVALID_INTERPOLATION_UNESCAPED,
    &INVALID_INTERPOLATION_NAME,
    &INVALID_INTERPOLATION_DUPLICATE,
    &INVALID_INTERPOLATION_UNUSED,
    &INVALID_INTERPOLATION_UNBOUND,
    &INVALID_PLURAL_NEWLINE,
    &INVALID_PLURAL_MISSING_COUNT,
    &INVALID_PLURAL_COUNT_TYPE,
];
static AUXILIARY_CONTRACTS: &[&DiagnosticAuxiliaryPresentationContract] = &[
    &MISSING_CHOICE_TARGET_HELP,
    &UNSUPPORTED_LINE_CHILD_RELATED,
    &UNSUPPORTED_LINE_CHILD_HELP,
    &UNSUPPORTED_CHOICE_CHILD_RELATED,
    &UNSUPPORTED_CHOICE_CHILD_HELP,
    &UNKNOWN_CHOICE_ECHO_HELP,
    &NON_FINITE_FLOAT_HELP,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}

pub(super) fn auxiliary_contracts()
-> impl Iterator<Item = &'static DiagnosticAuxiliaryPresentationContract> {
    AUXILIARY_CONTRACTS.iter().copied()
}
