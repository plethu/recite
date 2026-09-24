//! Parse native launch arguments before constructing a writer window.
use std::path::PathBuf;

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

impl Startup {
    /// Parse arguments after the executable name, including a positional desktop URL.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut project = None;
        let mut route = None;
        let mut examples = false;
        let mut design = false;
        let mut help = false;
        let mut version = false;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--project" => {
                    if project.is_some() {
                        return Err("--project may only be supplied once".into());
                    }
                    let path = args
                        .next()
                        .filter(|value| !value.is_empty() && !value.starts_with("--"))
                        .ok_or("--project needs a path")?;
                    project = Some(PathBuf::from(path));
                }
                "--route" => {
                    if route.is_some() {
                        return Err("A writer route may only be supplied once".into());
                    }
                    route = Some(
                        args.next()
                            .filter(|value| !value.is_empty() && !value.starts_with("--"))
                            .ok_or("--route needs a URL")?,
                    );
                }
                "--examples" => examples = true,
                "--design-system" => design = true,
                "--help" | "-h" => help = true,
                "--version" | "-V" => version = true,
                _ if arg.starts_with("recite://") => {
                    if route.replace(arg).is_some() {
                        return Err("A writer route may only be supplied once".into());
                    }
                }
                _ => return Err(format!("Unknown writer argument: {arg}")),
            }
        }
        if help || version {
            if help && version {
                return Err("Choose either --help or --version".into());
            }
            return Ok(if help { Self::Help } else { Self::Version });
        }
        if design {
            if examples || project.is_some() || route.is_some() {
                return Err("--design-system cannot open a project or route".into());
            }
            return Ok(Self::DesignSystem);
        }
        let linked_project = route
            .as_deref()
            .map(crate::navigation::project_from_route)
            .transpose()?
            .flatten();
        if examples {
            if project.is_some() || linked_project.is_some() {
                return Err("Example sessions cannot open a linked project".into());
            }
            return Ok(Self::Examples { route });
        }
        let project = match (project, linked_project) {
            (Some(explicit), Some(linked)) => {
                let explicit_root = canonical_project(&explicit)?;
                let linked_root = canonical_project(&linked)?;
                if explicit_root != linked_root {
                    return Err(format!(
                        "The link identifies another project: {}",
                        linked.display()
                    ));
                }
                Some(explicit_root)
            }
            (None, Some(linked)) => Some(canonical_project(&linked)?),
            (Some(explicit), None) => Some(canonical_project(&explicit)?),
            (None, None) if route.is_some() => {
                return Err(
                    "A writer link needs project= or --project (use --examples for example links)"
                        .into(),
                );
            }
            (None, None) => None,
        };
        Ok(Self::Writer { project, route })
    }
}

fn canonical_project(path: &std::path::Path) -> Result<PathBuf, String> {
    recite_config::discover_project(path)
        .map(|report| report.manifest().project_root().to_owned())
        .map_err(|error| format!("Could not open linked project {}: {error}", path.display()))
}

/// Project selected before the file-backed shell starts loading.
#[derive(Clone)]
pub struct InitialProject(pub Option<PathBuf>);

pub const HELP: &str = "Usage: recite-writer [--project PATH] [--route URL | recite://writer/...] [--examples] [--design-system]\n       recite-writer --help | --version";
