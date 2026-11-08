//! The closed set of commands that speak the structured protocol.
//!
//! `StructuredCommand` is the single place that ties a command's wire name to
//! the [`ErrorOperation`] reported when that command fails. Every `match` over
//! it, and the one over [`Command`] below, is exhaustive. A new CLI command
//! therefore does not compile until it is classified as structured or
//! human-only, and a structured one cannot be added without choosing its
//! operation. A test in this module checks that each wire name is a real
//! command path in the clap tree.

use serde::{Serialize, Serializer};

use super::errors::ErrorOperation;
use crate::args::{Command, IdCommand, OutputFormat, ProjectCommand, SchemaCommand};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(test, derive(strum::EnumIter))]
pub(crate) enum StructuredCommand {
    Validate,
    Compile,
    Extract,
    SchemaExport,
    Watch,
    Run,
    Trace,
}

impl StructuredCommand {
    /// The command path as typed, for example `schema export`.
    pub(crate) const fn wire_name(self) -> &'static str {
        match self {
            Self::Validate => "validate",
            Self::Compile => "compile",
            Self::Extract => "extract",
            Self::SchemaExport => "schema export",
            Self::Watch => "watch",
            Self::Run => "run",
            Self::Trace => "trace",
        }
    }

    /// The operation reported when the command itself fails before a more
    /// specific stage is known.
    pub(crate) const fn operation(self) -> ErrorOperation {
        match self {
            Self::Validate => ErrorOperation::Validate,
            Self::Compile => ErrorOperation::Compile,
            Self::Extract => ErrorOperation::Extract,
            Self::SchemaExport => ErrorOperation::ExportSchema,
            Self::Watch => ErrorOperation::Watch,
            Self::Run => ErrorOperation::Run,
            Self::Trace => ErrorOperation::Trace,
        }
    }
}

impl Serialize for StructuredCommand {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.wire_name())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StructuredInvocation {
    pub(crate) command: StructuredCommand,
    pub(crate) invocation_id: Option<String>,
}

impl Command {
    /// `Some` when the caller asked for structured output.
    ///
    /// No wildcard arms: adding a command or subcommand forces a decision here.
    /// Giving `id` or `project` commands structured output means adding a
    /// [`StructuredCommand`] variant, which in turn requires an
    /// [`ErrorOperation`].
    pub(crate) fn structured_invocation(&self) -> Option<StructuredInvocation> {
        let (command, output_format, invocation_id) = match self {
            Self::Validate(args) => (
                StructuredCommand::Validate,
                args.output_format,
                &args.invocation_id,
            ),
            Self::Compile(args) => (
                StructuredCommand::Compile,
                args.output_format,
                &args.invocation_id,
            ),
            Self::Extract(args) => (
                StructuredCommand::Extract,
                args.output_format,
                &args.invocation_id,
            ),
            Self::Schema(SchemaCommand::Export(args)) => (
                StructuredCommand::SchemaExport,
                args.output_format,
                &args.invocation_id,
            ),
            Self::Watch(args) => (
                StructuredCommand::Watch,
                args.output_format,
                &args.invocation_id,
            ),
            Self::Run(args) => (
                StructuredCommand::Run,
                args.output_format,
                &args.invocation_id,
            ),
            Self::Trace(args) => (
                StructuredCommand::Trace,
                args.runtime.output_format,
                &args.runtime.invocation_id,
            ),
            Self::Id(
                IdCommand::Check(_)
                | IdCommand::Assign(_)
                | IdCommand::Rename(_)
                | IdCommand::Reassign(_),
            )
            | Self::Schema(SchemaCommand::Inspect(_) | SchemaCommand::CheckFresh(_))
            | Self::Project(ProjectCommand::Validate(_) | ProjectCommand::CheckFresh(_))
            | Self::Explain(_)
            | Self::Play(_)
            | Self::Bench(_)
            | Self::Import(_) => return None,
        };

        (output_format == OutputFormat::Structured).then(|| StructuredInvocation {
            command,
            invocation_id: invocation_id.clone(),
        })
    }
}
