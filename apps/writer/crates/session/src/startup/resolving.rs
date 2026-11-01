use super::parsing::Arguments;
use super::*;
use std::path::Path;
impl Arguments {
    pub(super) fn resolve(self) -> Result<LaunchOptions, String> {
        if self.help || self.version {
            if self.help && self.version {
                return Err("Choose either --help or --version".into());
            }
            return Ok(LaunchOptions {
                intent: if self.help {
                    Startup::Help
                } else {
                    Startup::Version
                },
                document: None,
                editor: None,
            });
        }
        if self.editor.is_none() && !self.editor_args.is_empty() {
            return Err("--editor-arg requires --editor".into());
        }
        if self.design {
            if self.examples
                || self.project.is_some()
                || self.route.is_some()
                || self.document.is_some()
                || self.editor.is_some()
            {
                return Err(if self.document.is_some() || self.editor.is_some() {
                    "--design-system cannot open a document or external editor".into()
                } else {
                    "--design-system cannot open a project or route".into()
                });
            }
            return Ok(LaunchOptions {
                intent: Startup::DesignSystem,
                document: None,
                editor: None,
            });
        }
        let linked_project = self
            .route
            .as_deref()
            .map(crate::location::project_from_route)
            .transpose()?
            .flatten();
        if self.examples {
            if self.project.is_some()
                || linked_project.is_some()
                || self.document.is_some()
                || self.editor.is_some()
            {
                return Err(
                    "Example sessions cannot open a project, document or external editor".into(),
                );
            }
            return Ok(LaunchOptions {
                intent: Startup::Examples { route: self.route },
                document: None,
                editor: None,
            });
        }
        let project = resolve_project(self.project, linked_project, self.route.is_some())?;
        if let Some(document) = &self.document {
            let root = project.as_ref().ok_or("--document requires a project")?;
            validate_document(root, document, self.route.as_deref())?;
        }
        Ok(LaunchOptions {
            intent: Startup::Writer {
                project,
                route: self.route,
            },
            document: self.document,
            editor: self.editor.map(|program| EditorCommand {
                program,
                args: self.editor_args,
            }),
        })
    }
}
fn resolve_project(
    explicit: Option<PathBuf>,
    linked: Option<PathBuf>,
    has_route: bool,
) -> Result<Option<PathBuf>, String> {
    match (explicit, linked) {
        (Some(explicit), Some(linked)) => {
            let root = canonical_project(&explicit)?;
            if root != canonical_project(&linked)? {
                return Err(format!(
                    "The link identifies another project: {}",
                    linked.display()
                ));
            }
            Ok(Some(root))
        }
        (Some(path), None) | (None, Some(path)) => canonical_project(&path).map(Some),
        (None, None) if has_route => Err(
            "A writer link needs project= or --project (use --examples for example links)".into(),
        ),
        (None, None) => Ok(None),
    }
}
fn canonical_project(path: &Path) -> Result<PathBuf, String> {
    recite_config::discover_project(path)
        .map(|report| report.manifest().project_root().to_owned())
        .map_err(|error| format!("Could not open linked project {}: {error}", path.display()))
}
pub(crate) fn validate_document(
    root: &Path,
    document: &Path,
    route: Option<&str>,
) -> Result<(), String> {
    let report = recite_config::discover_project(root).map_err(|e| e.to_string())?;
    let path = root.join(document);
    let selected = report
        .documents()
        .iter()
        .find(|entry| entry.path() == path)
        .ok_or("The selected document is not in this project")?;
    if let Some(route) = route {
        let location: crate::location::Location = route.parse()?;
        if !location.document.is_empty() && location.document != selected.key().as_str() {
            return Err("--document and the writer link select different documents".into());
        }
    }
    Ok(())
}
