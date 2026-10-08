//! Error category policy shared by finite commands and project/watch mapping.
use std::path::Path;

use super::{ErrorCategory, ErrorCode, ErrorDetails, ErrorOperation, ErrorParts};

pub(super) fn generic<'a>(
    category: ErrorCategory,
    code: ErrorCode,
    operation: ErrorOperation,
    path: Option<&'a Path>,
) -> ErrorParts<'a> {
    (category, code, operation, path, None, None)
}

pub(super) fn compilation<'a>(
    code: ErrorCode,
    operation: ErrorOperation,
    path: Option<&'a Path>,
) -> ErrorParts<'a> {
    generic(ErrorCategory::Compilation, code, operation, path)
}

pub(super) fn input<'a>(
    code: ErrorCode,
    operation: ErrorOperation,
    path: Option<&'a Path>,
) -> ErrorParts<'a> {
    generic(ErrorCategory::Input, code, operation, path)
}

pub(super) fn internal<'a>(
    code: ErrorCode,
    operation: ErrorOperation,
    path: Option<&'a Path>,
) -> ErrorParts<'a> {
    generic(ErrorCategory::Internal, code, operation, path)
}

pub(super) fn localised<'a>(
    code: ErrorCode,
    operation: ErrorOperation,
    path: Option<&'a Path>,
) -> ErrorParts<'a> {
    generic(ErrorCategory::Localisation, code, operation, path)
}

pub(super) fn localised_details<'a>(
    code: ErrorCode,
    operation: ErrorOperation,
    path: Option<&'a Path>,
    details: ErrorDetails,
) -> ErrorParts<'a> {
    (
        ErrorCategory::Localisation,
        code,
        operation,
        path,
        None,
        Some(details),
    )
}

pub(super) fn asset<'a>(
    code: ErrorCode,
    operation: ErrorOperation,
    path: &'a Path,
) -> ErrorParts<'a> {
    generic(ErrorCategory::Asset, code, operation, Some(path))
}

pub(super) fn fixture<'a>(
    code: ErrorCode,
    operation: ErrorOperation,
    path: &'a Path,
) -> ErrorParts<'a> {
    generic(ErrorCategory::Fixture, code, operation, Some(path))
}

pub(super) fn fixture_details<'a>(
    code: ErrorCode,
    path: Option<&'a Path>,
    details: ErrorDetails,
) -> ErrorParts<'a> {
    (
        ErrorCategory::Fixture,
        code,
        ErrorOperation::SelectFixtureChoice,
        path,
        None,
        Some(details),
    )
}

pub(super) fn unsupported<'a>(
    code: ErrorCode,
    operation: ErrorOperation,
    path: Option<&'a Path>,
) -> ErrorParts<'a> {
    generic(ErrorCategory::Unsupported, code, operation, path)
}
