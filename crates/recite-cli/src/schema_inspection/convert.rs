use crate::error::CliError;

use super::model::SchemaInspectionProjection;
use super::summary;

impl SchemaInspectionProjection {
    pub(super) fn from_source(
        source: &recite_core::SchemaSource,
        path: String,
    ) -> Result<Self, CliError> {
        summary::from_source(source, path)
    }

    pub(super) fn from_generated(
        schema: &recite_core::ProjectSchema,
        path: String,
    ) -> Result<Self, CliError> {
        summary::from_generated(schema, path)
    }
}
