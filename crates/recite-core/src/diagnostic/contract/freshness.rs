use super::{DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticPresentationContract};

const ASSET: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "asset",
    DiagnosticArgumentType::String,
)];
const ASSET_SOURCE: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("asset", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("source", DiagnosticArgumentType::String),
];
const ASSET_VERSION: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("asset", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("version", DiagnosticArgumentType::Integer),
    DiagnosticArgumentSpec::new("expected", DiagnosticArgumentType::Integer),
];

const STALE_SOURCE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_FRESH001", "diagnostic-fresh-001", ASSET_SOURCE);
const STALE_SCHEMA: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_FRESH002", "diagnostic-fresh-002", ASSET);
const STALE_COMPILER: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_FRESH003", "diagnostic-fresh-003", ASSET_VERSION);

static CONTRACTS: &[&DiagnosticPresentationContract] =
    &[&STALE_SOURCE, &STALE_SCHEMA, &STALE_COMPILER];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}
