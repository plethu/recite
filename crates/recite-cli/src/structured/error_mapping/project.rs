//! Preserve declared and resolved schema paths in the command error contract.
use std::path::Path;

use recite_config::ProjectSchemaError;

use super::{ErrorCategory, ErrorCode, ErrorParts, generic};

pub(super) fn schema<'a>(
    source: &'a ProjectSchemaError,
    fallback_path: Option<&'a Path>,
) -> ErrorParts<'a> {
    match source {
        ProjectSchemaError::Read { path, .. } => {
            generic(ErrorCategory::Io, ErrorCode::Read, "read", Some(path))
        }
        ProjectSchemaError::InvalidPath { path, .. } => generic(
            ErrorCategory::Schema,
            ErrorCode::ProjectSchema,
            "resolve_schema",
            Some(path),
        ),
        ProjectSchemaError::OutsideProject { declared, resolved } => (
            ErrorCategory::Schema,
            ErrorCode::ProjectSchema,
            "resolve_schema",
            Some(declared),
            Some(resolved),
            None,
        ),
        _ => generic(
            ErrorCategory::Schema,
            ErrorCode::ProjectSchema,
            "load_schema",
            fallback_path,
        ),
    }
}
