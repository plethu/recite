use crate::i18n::{Messages, MsgId};

/// Something a command takes from the project when it is not passed, and could not.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum ProjectDefaultError {
    /// A command that defaults to the project's sources ran where no project manifest exists.
    NoInputsOutsideProject,
    /// `--only metadata` found neither `--schema` nor a project schema.
    SchemaRequired,
    /// No `--block` and no single project scene that uses the asset.
    BlockRequired { asset: String },
    /// No `--block`, and several project scenes start the asset at different blocks.
    BlockAmbiguous { asset: String, blocks: Vec<String> },
    /// `schema check-fresh` was given only one of `--expected` and `--actual`.
    SchemaFreshPair,
    /// `schema check-fresh` in a project whose manifest declares no schema.
    SchemaFreshNoSchema,
    /// `schema check-fresh` in a project with no registered producer.
    SchemaFreshNoProducer,
    /// The registered producer failed to export a schema to compare.
    SchemaProducer { detail: String },
}

impl From<ProjectDefaultError> for super::CliError {
    fn from(error: ProjectDefaultError) -> Self {
        Self::ProjectDefault(error)
    }
}

impl std::fmt::Display for ProjectDefaultError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoInputsOutsideProject => formatter.write_str(
                "no paths given and no recite.project.toml found here or above; pass the .recite files or directories to read",
            ),
            Self::SchemaRequired => formatter.write_str(
                "--only metadata needs a schema: pass --schema, or declare [project].schema in recite.project.toml",
            ),
            Self::BlockRequired { asset } => write!(
                formatter,
                "pass --block: no scene in recite.project.toml uses {asset}"
            ),
            Self::BlockAmbiguous { asset, blocks } => write!(
                formatter,
                "pass --block: scenes in recite.project.toml start {asset} at {}",
                blocks.join(", ")
            ),
            Self::SchemaFreshPair => formatter.write_str(
                "pass both --expected and --actual, or neither to check the project here",
            ),
            Self::SchemaFreshNoSchema => {
                formatter.write_str("recite.project.toml declares no [project].schema to check")
            }
            Self::SchemaFreshNoProducer => formatter.write_str(
                "no recite.producer.toml registers a schema producer here; pass --expected and --actual to compare two exports",
            ),
            Self::SchemaProducer { detail } => write!(
                formatter,
                "the schema producer could not export a schema: {detail}"
            ),
        }
    }
}

impl ProjectDefaultError {
    pub(crate) fn to_user_message(&self, messages: &Messages) -> String {
        match self {
            Self::NoInputsOutsideProject => messages.text(MsgId::CliErrorNoInputsOutsideProject),
            Self::SchemaRequired => messages.text(MsgId::CliErrorSchemaRequired),
            Self::BlockRequired { asset } => {
                messages.format(MsgId::CliErrorBlockRequired, [("asset", asset.clone())])
            }
            Self::BlockAmbiguous { asset, blocks } => messages.format(
                MsgId::CliErrorBlockAmbiguous,
                [("asset", asset.clone()), ("blocks", blocks.join(", "))],
            ),
            Self::SchemaFreshPair => messages.text(MsgId::CliErrorSchemaFreshPair),
            Self::SchemaFreshNoSchema => messages.text(MsgId::CliErrorSchemaFreshNoSchema),
            Self::SchemaFreshNoProducer => messages.text(MsgId::CliErrorSchemaFreshNoProducer),
            Self::SchemaProducer { detail } => {
                messages.format(MsgId::CliErrorSchemaProducer, [("detail", detail.clone())])
            }
        }
    }
}
