use super::{DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticPresentationContract};

const DETAIL: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "detail",
    DiagnosticArgumentType::String,
)];
const INVALID: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_IMPORT001", "diagnostic-import-001", DETAIL);
const UNSUPPORTED: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_IMPORT002", "diagnostic-import-002", DETAIL);
const LOSS: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_IMPORT003", "diagnostic-import-003", DETAIL);
static CONTRACTS: &[&DiagnosticPresentationContract] = &[&INVALID, &UNSUPPORTED, &LOSS];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}
