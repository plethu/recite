//! Explicit project-owned producer invocation; loading never runs a command.
use crate::ProducerIdentity;
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};

pub const PRODUCER_REGISTRATION_FILE: &str = "recite.producer.toml";

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProducerRegistrationError {
    #[error("Invalid producer registration: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("Invalid producer registration: {0}")]
    Invalid(String),
}

/// Validated adapter capabilities. Commands use argv, never implicit shell parsing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProducerRegistration {
    producer: ProducerIdentity,
    generate: ProducerCommand,
    editor: Option<ProducerCommand>,
    sources: Vec<DeclarationSource>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerCommand {
    program: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default = "root_directory")]
    directory: String,
}
fn root_directory() -> String {
    ".".into()
}
impl ProducerCommand {
    pub fn program(&self) -> &str {
        &self.program
    }
    pub fn args(&self) -> &[String] {
        &self.args
    }
    pub fn directory(&self) -> &str {
        &self.directory
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclarationSource {
    kind: String,
    name: String,
    file: String,
    line: u32,
    #[serde(default = "first_column")]
    column: u32,
}
fn first_column() -> u32 {
    1
}
impl DeclarationSource {
    pub fn kind(&self) -> &str {
        &self.kind
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn file(&self) -> &str {
        &self.file
    }
    pub const fn line(&self) -> u32 {
        self.line
    }
    pub const fn column(&self) -> u32 {
        self.column
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    version: u32,
    producer: ProducerIdentity,
    generate: ProducerCommand,
    editor: Option<ProducerCommand>,
    #[serde(default)]
    sources: Vec<DeclarationSource>,
}
impl ProducerRegistration {
    pub fn parse(text: &str) -> Result<Self, ProducerRegistrationError> {
        let wire: Wire = toml::from_str(text)?;
        let invalid = |s: &str| ProducerRegistrationError::Invalid(s.into());
        if wire.version != 1 {
            return Err(invalid("version must be 1"));
        }
        validate_command(&wire.generate, &["{output}"])?;
        if !wire
            .generate
            .args
            .iter()
            .any(|arg| arg.contains("{output}"))
        {
            return Err(invalid(
                "generate.args must include {output}, the staged output path",
            ));
        }
        if let Some(editor) = &wire.editor {
            validate_command(editor, &["{file}", "{line}", "{column}"])?;
            if !editor.args.iter().any(|a| a.contains("{file}")) {
                return Err(invalid("editor.args must include {file}"));
            }
        }
        let mut keys = BTreeSet::new();
        for source in &wire.sources {
            if !matches!(
                source.kind.as_str(),
                "condition"
                    | "effect"
                    | "speaker"
                    | "registry"
                    | "type"
                    | "availability_reason"
                    | "metadata_domain"
                    | "metadata"
                    | "projection_query"
                    | "presentation_projector"
                    | "markup"
            ) || source.name.trim().is_empty()
                || source.line == 0
                || source.column == 0
                || !relative(&source.file)
                || !keys.insert((&source.kind, &source.name))
            {
                return Err(invalid(
                    "source locations need a known kind, unique name, project-relative file and positive line/column",
                ));
            }
        }
        Ok(Self {
            producer: wire.producer,
            generate: wire.generate,
            editor: wire.editor,
            sources: wire.sources,
        })
    }
    pub fn producer(&self) -> &ProducerIdentity {
        &self.producer
    }
    pub fn generate(&self) -> &ProducerCommand {
        &self.generate
    }
    pub fn editor(&self) -> Option<&ProducerCommand> {
        self.editor.as_ref()
    }
    pub fn sources(&self) -> &[DeclarationSource] {
        &self.sources
    }
}
fn relative(value: &str) -> bool {
    !value.is_empty()
        && Path::new(value)
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}
fn validate_command(
    command: &ProducerCommand,
    placeholders: &[&str],
) -> Result<(), ProducerRegistrationError> {
    let valid_args = command.args.iter().all(|arg| {
        let mut remaining = arg.clone();
        for placeholder in placeholders {
            remaining = remaining.replace(placeholder, "");
        }
        !remaining.contains(['{', '}', '\0'])
    });
    if command.program.trim().is_empty()
        || command.program.contains(['{', '}', '\0'])
        || !relative(&command.directory)
        || !valid_args
    {
        return Err(ProducerRegistrationError::Invalid(
            "commands need a program, project-relative directory and only documented placeholders"
                .into(),
        ));
    }
    Ok(())
}
