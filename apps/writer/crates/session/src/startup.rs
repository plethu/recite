//! One native launch intent; paths and external process arguments stay native.
use std::{ffi::OsString, path::PathBuf};
mod parsing;
mod resolving;
pub(crate) use resolving::validate_document;

/// The work the native host should perform for one invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Startup {
    Writer {
        project: Option<PathBuf>,
        route: Option<String>,
    },
    Examples {
        route: Option<String>,
    },
    DesignSystem,
    Help,
    Version,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorCommand {
    pub program: OsString,
    pub args: Vec<OsString>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchOptions {
    pub intent: Startup,
    pub document: Option<PathBuf>,
    pub editor: Option<EditorCommand>,
}
impl LaunchOptions {
    pub fn route(&self) -> Result<Option<String>, String> {
        if self.document.is_some() {
            return self
                .location()
                .map(|location| location.map(|location| format!("recite://writer{location}")));
        }
        Ok(match &self.intent {
            Startup::Writer { route, .. } | Startup::Examples { route } => route.clone(),
            _ => None,
        })
    }
    /// Resolve the optional document selector into the same location used by links.
    pub fn location(&self) -> Result<Option<crate::location::Location>, String> {
        let (project, route) = match &self.intent {
            Startup::Writer { project, route } => (project.as_deref(), route.as_deref()),
            Startup::Examples { route } => (None, route.as_deref()),
            _ => return Ok(None),
        };
        if route.is_none() && self.document.is_none() {
            return Ok(None);
        }
        let mut location: crate::location::Location = route
            .map(str::parse)
            .transpose()
            .map_err(str::to_owned)?
            .unwrap_or_default();
        if let Some(document) = &self.document {
            let project = project.ok_or("A selected document requires a project")?;
            let report = recite_config::discover_project(project).map_err(|e| e.to_string())?;
            let path = project.join(document);
            let entry = report
                .documents()
                .iter()
                .find(|entry| entry.path() == path)
                .ok_or("The selected document is not in this project")?;
            location.document = entry.key().as_str().to_owned();
        }
        Ok(Some(location))
    }
}
impl Startup {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let options = LaunchOptions::parse(args.into_iter().map(OsString::from))?;
        if options.document.is_some() || options.editor.is_some() {
            return Err(
                "Use LaunchOptions to preserve document and external editor options.".into(),
            );
        }
        Ok(options.intent)
    }
}
pub const HELP: &str = "Usage: recite-writer [--project PATH] [--document PATH] [--route URL | recite://writer/...] [--editor PROGRAM] [--editor-arg ARG] [--examples] [--design-system]\n       recite-writer --help | --version\n\nEditor arguments are literal process arguments; the source file path is appended.";

#[cfg(test)]
mod tests;
