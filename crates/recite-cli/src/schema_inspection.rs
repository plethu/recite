//! The CLI-owned JSON projection of a canonical schema summary.
//!
//! This module is deliberately a presentation boundary.  Input loading stays
//! with the authoritative `SchemaSource` and generated-manifest loaders; the
//! DTOs below never parse schema documents or execute a producer.

use std::fs;
use std::io::Write;

use crate::args::InspectSchemaArgs;
use crate::diagnostics::report_diagnostics;
use crate::error::CliError;
use crate::fs::display_path;
use crate::i18n::Messages;

pub(crate) mod error;

mod capabilities;
mod convert;
mod definitions;
mod fingerprints;
mod freshness;
mod input;
mod model;
mod provenance;
mod summary;

use self::error::SchemaInspectionError;
use input::InputFormat;
use model::SchemaInspectionProjection;

/// Version of the CLI inspection projection, independent of schema and
/// generated-manifest versions.
pub(crate) const INSPECTION_FORMAT_VERSION: u32 = 1;

/// Load and project one explicitly supported schema input.
pub(crate) fn run(
    args: InspectSchemaArgs,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let format = InputFormat::from_path(&args.schema).ok_or_else(|| {
        CliError::SchemaInspection(SchemaInspectionError::UnsupportedFormat {
            path: args.schema.clone(),
            format: args
                .schema
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("<none>")
                .to_owned(),
        })
    })?;
    let source = fs::read_to_string(&args.schema).map_err(|source| CliError::Read {
        path: args.schema.clone(),
        source,
    })?;
    let file = display_path(&args.schema);
    let loaded = match format {
        InputFormat::StandaloneToml => {
            let report = recite_core::SchemaSource::load_str(file.clone(), &source);
            if !report.diagnostics.is_empty() {
                report_diagnostics(stderr, messages, report.diagnostics.iter())?;
                return Err(CliError::SchemaInspection(
                    SchemaInspectionError::Malformed {
                        path: args.schema,
                        format: format.name(),
                    },
                ));
            }
            let Some(source) = report.source else {
                return Err(CliError::SchemaInspection(
                    SchemaInspectionError::Malformed {
                        path: args.schema,
                        format: format.name(),
                    },
                ));
            };
            SchemaInspectionProjection::from_source(&source, file)
        }
        InputFormat::GeneratedJson => {
            let report = recite_core::load_schema_manifest_str(file.clone(), &source);
            if !report.diagnostics.is_empty() {
                report_diagnostics(stderr, messages, report.diagnostics.iter())?;
                return Err(CliError::SchemaInspection(
                    SchemaInspectionError::Malformed {
                        path: args.schema,
                        format: format.name(),
                    },
                ));
            }
            let Some(schema) = report.schema else {
                return Err(CliError::SchemaInspection(
                    SchemaInspectionError::Malformed {
                        path: args.schema,
                        format: format.name(),
                    },
                ));
            };
            SchemaInspectionProjection::from_generated(&schema, file)
        }
    }?;

    serde_json::to_writer_pretty(&mut *stdout, &loaded)
        .map_err(SchemaInspectionError::Json)
        .map_err(CliError::SchemaInspection)?;
    writeln!(stdout)?;
    Ok(())
}
