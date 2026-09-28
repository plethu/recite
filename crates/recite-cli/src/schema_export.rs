//! Canonical schema export for engine authoring tools.

use std::fs;
use std::io::Write;
use std::path::Path;

use recite_core::Diagnostic;
use recite_core::schema::{
    ProducerIdentity, SchemaSource, export_schema_manifest_json,
    export_schema_manifest_json_with_producer, load_schema_manifest_str,
};

use crate::args::ExportSchemaArgs;
use crate::diagnostics::report_diagnostics;
use crate::error::CliError;
use crate::fs::{display_path, reject_output_input_alias, write_staged};
use crate::i18n::Messages;
use crate::schema_inspection::error::SchemaInspectionError;

pub(crate) enum PreparedExport {
    Ready(String),
    Diagnostics(Vec<Diagnostic>),
}

pub(crate) fn prepare(args: &ExportSchemaArgs) -> Result<PreparedExport, CliError> {
    reject_output_input_alias(&args.output, std::slice::from_ref(&args.schema))?;
    let source = fs::read_to_string(&args.schema).map_err(|source| CliError::Read {
        path: args.schema.clone(),
        source,
    })?;
    let file = display_path(&args.schema);
    let report = match args
        .schema
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("toml") => {
            let report = SchemaSource::load_str(file, &source);
            (
                report.source.map(|source| source.schema().clone()),
                report.diagnostics,
            )
        }
        Some("json") => {
            let report = load_schema_manifest_str(file, &source);
            (report.schema, report.diagnostics)
        }
        _ => {
            return Err(CliError::SchemaInspection(
                SchemaInspectionError::UnsupportedFormat {
                    path: args.schema.clone(),
                    format: args
                        .schema
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .unwrap_or("<none>")
                        .to_owned(),
                },
            ));
        }
    };
    if !report.1.is_empty() {
        return Ok(PreparedExport::Diagnostics(report.1));
    }
    let Some(schema) = report.0 else {
        return Ok(PreparedExport::Diagnostics(report.1));
    };
    let exported = match (&args.producer_kind, &args.producer_id) {
        (Some(kind), Some(id)) => {
            let producer = ProducerIdentity::new(kind.clone(), id.clone()).map_err(|error| {
                CliError::SchemaInspection(SchemaInspectionError::InvalidSummary {
                    reason: format!("invalid producer identity: {error}"),
                })
            })?;
            export_schema_manifest_json_with_producer(&schema, producer)
        }
        (None, None) => export_schema_manifest_json(&schema),
        _ => unreachable!("clap requires paired producer identity arguments"),
    };
    match exported {
        Ok(json) => Ok(PreparedExport::Ready(json)),
        Err(diagnostics) => Ok(PreparedExport::Diagnostics(diagnostics)),
    }
}

pub(crate) fn write(output: &Path, json: &str) -> Result<(), CliError> {
    write_staged(output, json.as_bytes())
}

pub(crate) fn run(
    args: ExportSchemaArgs,
    _stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    match prepare(&args)? {
        PreparedExport::Ready(json) => write(&args.output, &json),
        PreparedExport::Diagnostics(diagnostics) => {
            report_diagnostics(stderr, messages, diagnostics.iter())?;
            Err(CliError::Diagnostics)
        }
    }
}
