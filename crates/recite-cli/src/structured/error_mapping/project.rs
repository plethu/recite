//! Preserve declared and resolved schema paths in the command error contract.
use std::path::Path;

use recite_config::ProjectSchemaError;

use super::{ErrorCategory, ErrorCode, ErrorOperation, ErrorParts, generic};

#[path = "project/tests.rs"]
#[cfg(test)]
mod tests;

pub(super) fn schema<'a>(
    source: &'a ProjectSchemaError,
    fallback_path: Option<&'a Path>,
) -> ErrorParts<'a> {
    match source {
        ProjectSchemaError::Read { path, .. } => generic(
            ErrorCategory::Io,
            ErrorCode::Read,
            ErrorOperation::Read,
            Some(path),
        ),
        ProjectSchemaError::InvalidPath { path, .. } => generic(
            ErrorCategory::Schema,
            ErrorCode::ProjectSchema,
            ErrorOperation::ResolveSchema,
            Some(path),
        ),
        ProjectSchemaError::OutsideProject { declared, resolved } => (
            ErrorCategory::Schema,
            ErrorCode::ProjectSchema,
            ErrorOperation::ResolveSchema,
            Some(declared),
            Some(resolved),
            None,
        ),
        _ => generic(
            ErrorCategory::Schema,
            ErrorCode::ProjectSchema,
            ErrorOperation::LoadSchema,
            fallback_path,
        ),
    }
}
