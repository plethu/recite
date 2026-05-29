use crate::DiagnosticCode;

pub(crate) const MALFORMED_SHAPE: DiagnosticCode = DiagnosticCode::new_static("RECITE_SCHEMA001");
pub(crate) const UNSUPPORTED_VERSION: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_SCHEMA002");
pub(crate) const DUPLICATE_DEFINITION: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_SCHEMA003");
pub(crate) const INVALID_TYPE_REFERENCE: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_SCHEMA004");
