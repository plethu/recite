use super::{DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticPresentationContract};

const DETAIL: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "detail",
    DiagnosticArgumentType::String,
)];

const MISSING_MANIFEST: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG101", "diagnostic-config-101", DETAIL);
const MANIFEST_READ: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG102", "diagnostic-config-102", DETAIL);
const MANIFEST_MALFORMED: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG103", "diagnostic-config-103", DETAIL);
const MANIFEST_VERSION: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG104", "diagnostic-config-104", DETAIL);
const INVALID_ROOT: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG105", "diagnostic-config-105", DETAIL);
const INVALID_EXCLUDE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG106", "diagnostic-config-106", DETAIL);
const ROOT_MISSING: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG107", "diagnostic-config-107", DETAIL);
const ROOT_READ: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG108", "diagnostic-config-108", DETAIL);
const ROOT_OUTSIDE_PROJECT: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG109", "diagnostic-config-109", DETAIL);
const DUPLICATE_ROOT: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG110", "diagnostic-config-110", DETAIL);
const OVERLAPPING_ROOT: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG111", "diagnostic-config-111", DETAIL);
const DISCOVERY_READ: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG112", "diagnostic-config-112", DETAIL);
const NON_UTF8_PATH: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG113", "diagnostic-config-113", DETAIL);
const FILE_OUTSIDE_PROJECT: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG114", "diagnostic-config-114", DETAIL);
const NON_UTF8_SOURCE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG115", "diagnostic-config-115", DETAIL);
const ROOT_NOT_DIRECTORY: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG116", "diagnostic-config-116", DETAIL);
const INVALID_DOCUMENT_KEY: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_CONFIG117", "diagnostic-config-117", DETAIL);

static CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &MISSING_MANIFEST,
    &MANIFEST_READ,
    &MANIFEST_MALFORMED,
    &MANIFEST_VERSION,
    &INVALID_ROOT,
    &INVALID_EXCLUDE,
    &ROOT_MISSING,
    &ROOT_READ,
    &ROOT_OUTSIDE_PROJECT,
    &DUPLICATE_ROOT,
    &OVERLAPPING_ROOT,
    &DISCOVERY_READ,
    &NON_UTF8_PATH,
    &FILE_OUTSIDE_PROJECT,
    &NON_UTF8_SOURCE,
    &ROOT_NOT_DIRECTORY,
    &INVALID_DOCUMENT_KEY,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}
