use super::{DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticPresentationContract};

const ASSET: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "asset",
    DiagnosticArgumentType::String,
)];
const ASSET_SOURCES: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("asset", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("sources", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("count", DiagnosticArgumentType::Integer),
];
const ASSET_VERSION: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("asset", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("version", DiagnosticArgumentType::Integer),
    DiagnosticArgumentSpec::new("expected", DiagnosticArgumentType::Integer),
];

const STALE_SOURCE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_FRESH001", "diagnostic-fresh-001", ASSET_SOURCES);
const STALE_SCHEMA: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_FRESH002", "diagnostic-fresh-002", ASSET);
const STALE_COMPILER: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_FRESH003", "diagnostic-fresh-003", ASSET_VERSION);

const ADDED_SOURCE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_FRESH004", "diagnostic-fresh-004", ASSET_SOURCES);

static CONTRACTS: &[&DiagnosticPresentationContract] =
    &[&STALE_SOURCE, &STALE_SCHEMA, &STALE_COMPILER, &ADDED_SOURCE];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}
