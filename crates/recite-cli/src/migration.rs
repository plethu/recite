use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use clap::{Args, ValueEnum};
use recite_import::{FieldMapping, ImportRequest, ImportStatus, SourceFamily};

use crate::{
    error::CliError,
    fs::load_optional_schema,
    i18n::{Messages, MsgId},
};

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum Family {
    Json,
    Csv,
    Twee,
    Ink,
    Yarn,
}

impl From<Family> for SourceFamily {
    fn from(value: Family) -> Self {
        match value {
            Family::Json => Self::Json,
            Family::Csv => Self::Csv,
            Family::Twee => Self::Twee,
            Family::Ink => Self::Ink,
            Family::Yarn => Self::Yarn,
        }
    }
}

#[derive(Debug, Args)]
pub(crate) struct ImportArgs {
    pub(crate) input: PathBuf,
    #[arg(long, value_enum)]
    pub(crate) from: Family,
    /// Stable source path used in provenance and generated IDs (defaults to input path).
    #[arg(long, value_parser = clap::builder::NonEmptyStringValueParser::new())]
    pub(crate) source_id: Option<String>,
    /// Omit the default block marker when importing another file into a project.
    #[arg(long)]
    pub(crate) no_default: bool,
    /// JSON file naming the source fields; required for JSON and CSV input.
    #[arg(long)]
    pub(crate) mapping: Option<PathBuf>,
    #[arg(long)]
    pub(crate) schema: Option<PathBuf>,
    /// New directory for report.json and validated imported.recite.
    #[arg(long)]
    pub(crate) output_dir: Option<PathBuf>,
    /// Permit writing a partial conversion with explicit skipped constructs.
    #[arg(long, requires = "output_dir")]
    pub(crate) accept_partial: bool,
}

pub(crate) fn run(
    args: ImportArgs,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let source = read(&args.input)?;
    let mapping: Option<FieldMapping> = args
        .mapping
        .as_deref()
        .map(|path| serde_json::from_str(&read(path)?).map_err(CliError::ImportJson))
        .transpose()?;
    let schema = load_optional_schema(args.schema.as_deref(), stderr, messages)?;
    let file = match args.source_id.as_deref() {
        Some(id) => id,
        None => args.input.to_str().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "migration input needs a UTF-8 path or --source-id",
            )
        })?,
    };
    let report = recite_import::import(ImportRequest {
        family: args.from.into(),
        file,
        source: &source,
        mapping: mapping.as_ref(),
        schema: schema.as_ref(),
        default_block: !args.no_default,
    })
    .map_err(CliError::Import)?;
    let mut encoded = serde_json::to_vec_pretty(&report).map_err(CliError::ImportJson)?;
    encoded.push(b'\n');
    stdout.write_all(&encoded)?;
    for (construct, counts) in report.counts_by_construct() {
        writeln!(
            stderr,
            "{}: {construct}: {}",
            report.file,
            messages.format(
                MsgId::CliImportSummary,
                [
                    ("records", counts.generated.to_string()),
                    ("issues", counts.review_items.to_string()),
                ]
            )
        )?;
    }
    let writable = report.status == ImportStatus::Complete
        || (report.status == ImportStatus::Partial && args.accept_partial);
    if let Some(destination) = args.output_dir {
        if !writable {
            return Err(CliError::Diagnostics);
        }
        publish(&destination, &encoded, report.source.as_bytes())?;
    }
    if report.status == ImportStatus::Invalid {
        return Err(CliError::Diagnostics);
    }
    Ok(())
}

fn read(path: &Path) -> Result<String, CliError> {
    fs::read_to_string(path).map_err(|source| CliError::Read {
        path: path.to_owned(),
        source,
    })
}

fn publish(destination: &Path, report: &[u8], source: &[u8]) -> Result<(), CliError> {
    // create_dir is exclusive: existing files/directories/symlinks are never
    // overwritten. A failed write leaves its new directory for inspection.
    fs::create_dir(destination).map_err(|source| CliError::Write {
        path: destination.to_owned(),
        source,
    })?;
    for (name, bytes) in [("report.json", report), ("imported.recite", source)] {
        let path = destination.join(name);
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| CliError::Write {
                path: path.clone(),
                source,
            })?;
        output
            .write_all(bytes)
            .and_then(|()| output.sync_all())
            .map_err(|source| CliError::Write { path, source })?;
    }
    Ok(())
}
