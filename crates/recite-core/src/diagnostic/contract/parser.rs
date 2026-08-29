use super::{DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticPresentationContract};

const NO_ARGUMENTS: &[DiagnosticArgumentSpec] = &[];
const PARSE_REASON_ARGUMENTS: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "reason",
    DiagnosticArgumentType::String,
)];
const CHARACTER_ARGUMENTS: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "character",
    DiagnosticArgumentType::String,
)];

const PARSE001: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PARSE001", "diagnostic-parse-001", NO_ARGUMENTS);
const PARSE002: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PARSE002", "diagnostic-parse-002", NO_ARGUMENTS);
const PARSE003: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PARSE003", "diagnostic-parse-003", NO_ARGUMENTS);
const PARSE005: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PARSE005", "diagnostic-parse-005", NO_ARGUMENTS);
const PARSE007: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PARSE007", "diagnostic-parse-007", NO_ARGUMENTS);
const PARSE008: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PARSE008", "diagnostic-parse-008", NO_ARGUMENTS);
const PARSE010: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PARSE010", "diagnostic-parse-010", NO_ARGUMENTS);
const PARSE011: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PARSE011", "diagnostic-parse-011", NO_ARGUMENTS);
const PARSE012: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_PARSE012",
    "diagnostic-parse-012",
    PARSE_REASON_ARGUMENTS,
);
const PARSE012_UNEXPECTED_CHARACTER: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_PARSE012",
        "diagnostic-parse-012-unexpected-character",
        CHARACTER_ARGUMENTS,
    );
const PARSE013: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_PARSE013",
    "diagnostic-parse-013",
    PARSE_REASON_ARGUMENTS,
);
const PARSE013_UNEXPECTED_CHARACTER: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_PARSE013",
        "diagnostic-parse-013-unexpected-character",
        CHARACTER_ARGUMENTS,
    );
const PARSE014: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PARSE014", "diagnostic-parse-014", NO_ARGUMENTS);
const PARSE015: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PARSE015", "diagnostic-parse-015", NO_ARGUMENTS);
const PARSE016: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PARSE016", "diagnostic-parse-016", NO_ARGUMENTS);
const PARSE017: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PARSE017", "diagnostic-parse-017", NO_ARGUMENTS);
const PARSE018: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PARSE018", "diagnostic-parse-018", NO_ARGUMENTS);

static CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &PARSE001,
    &PARSE002,
    &PARSE003,
    &PARSE005,
    &PARSE007,
    &PARSE008,
    &PARSE010,
    &PARSE011,
    &PARSE012,
    &PARSE012_UNEXPECTED_CHARACTER,
    &PARSE013,
    &PARSE013_UNEXPECTED_CHARACTER,
    &PARSE014,
    &PARSE015,
    &PARSE016,
    &PARSE017,
    &PARSE018,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}
