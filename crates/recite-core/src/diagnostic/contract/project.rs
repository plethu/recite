use super::{
    DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticAuxiliaryPresentationContract,
    DiagnosticPresentationContract,
};

const DETAIL: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "detail",
    DiagnosticArgumentType::String,
)];
const SCENE_ID: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "scene_id",
    DiagnosticArgumentType::String,
)];
const SCENE_ASSET: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("scene_id", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("asset", DiagnosticArgumentType::String),
];
const SCENE_BLOCK: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("scene_id", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("block", DiagnosticArgumentType::String),
];
const ASSET_SOURCE: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("asset", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("source", DiagnosticArgumentType::String),
];
const ASSET_VERSION: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("asset", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("version", DiagnosticArgumentType::Integer),
];
const MALFORMED_ASSET: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("scene_id", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("asset", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("detail", DiagnosticArgumentType::String),
];
const SCENE_PARTICIPANT: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("scene_id", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("participant", DiagnosticArgumentType::String),
];
const SCENE_PARTICIPANT_ASSET: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("scene_id", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("participant", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("asset", DiagnosticArgumentType::String),
];

const MALFORMED_MANIFEST: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PROJECT001", "diagnostic-project-001", DETAIL);
const DUPLICATE_SCENE_ID: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PROJECT002", "diagnostic-project-002", SCENE_ID);
const MISSING_COMPILED_ASSET: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PROJECT003", "diagnostic-project-003", SCENE_ASSET);
const UNKNOWN_START_BLOCK: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PROJECT004", "diagnostic-project-004", SCENE_BLOCK);
const MISSING_PARTICIPANTS: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PROJECT005", "diagnostic-project-005", SCENE_ID);
const MISSING_SOURCE_ASSET: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_PROJECT006",
    "diagnostic-project-006",
    ASSET_SOURCE,
);
const UNSUPPORTED_ASSET_VERSION: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_PROJECT007",
        "diagnostic-project-007",
        ASSET_VERSION,
    );
const MALFORMED_COMPILED_ASSET: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_PROJECT007",
        "diagnostic-project-007-malformed",
        MALFORMED_ASSET,
    );
const UNKNOWN_PARTICIPANT: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_PROJECT008",
    "diagnostic-project-008",
    SCENE_PARTICIPANT,
);
const COMPILED_ASSET_PARTICIPANT: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_PROJECT008",
        "diagnostic-project-008-compiled-asset",
        SCENE_PARTICIPANT_ASSET,
    );

const DUPLICATE_SCENE_RELATED: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-project-002-related", &[]);

static CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &MALFORMED_MANIFEST,
    &DUPLICATE_SCENE_ID,
    &MISSING_COMPILED_ASSET,
    &UNKNOWN_START_BLOCK,
    &MISSING_PARTICIPANTS,
    &MISSING_SOURCE_ASSET,
    &UNSUPPORTED_ASSET_VERSION,
    &MALFORMED_COMPILED_ASSET,
    &UNKNOWN_PARTICIPANT,
    &COMPILED_ASSET_PARTICIPANT,
];
static AUXILIARY_CONTRACTS: &[&DiagnosticAuxiliaryPresentationContract] =
    &[&DUPLICATE_SCENE_RELATED];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}

pub(super) fn auxiliary_contracts()
-> impl Iterator<Item = &'static DiagnosticAuxiliaryPresentationContract> {
    AUXILIARY_CONTRACTS.iter().copied()
}
