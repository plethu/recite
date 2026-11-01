use super::*;
#[derive(Default)]
pub(super) struct Arguments {
    pub project: Option<PathBuf>,
    pub route: Option<String>,
    pub document: Option<PathBuf>,
    pub editor: Option<OsString>,
    pub editor_args: Vec<OsString>,
    pub examples: bool,
    pub design: bool,
    pub help: bool,
    pub version: bool,
}
impl LaunchOptions {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut parsed = Arguments::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--project") => unique(
                    &mut parsed.project,
                    PathBuf::from(value(&mut args, "--project")?),
                    "--project",
                )?,
                Some("--document") => unique(
                    &mut parsed.document,
                    PathBuf::from(value(&mut args, "--document")?),
                    "--document",
                )?,
                Some("--route") => {
                    let route = value(&mut args, "--route")?
                        .into_string()
                        .map_err(|_| "A writer route must be UTF-8")?;
                    unique(&mut parsed.route, route, "A writer route")?;
                }
                Some("--editor") => unique(
                    &mut parsed.editor,
                    value(&mut args, "--editor")?,
                    "--editor",
                )?,
                Some("--editor-arg") => parsed.editor_args.push(value(&mut args, "--editor-arg")?),
                Some("--examples") => flag(&mut parsed.examples, "--examples")?,
                Some("--design-system") => flag(&mut parsed.design, "--design-system")?,
                Some("--help" | "-h") => parsed.help = true,
                Some("--version" | "-V") => parsed.version = true,
                Some(route) if route.starts_with("recite://") => {
                    unique(&mut parsed.route, route.to_owned(), "A writer route")?
                }
                _ => {
                    return Err(format!(
                        "Unknown writer argument: {}",
                        arg.to_string_lossy()
                    ));
                }
            }
        }
        parsed.resolve()
    }
}
fn unique<T>(slot: &mut Option<T>, value: T, name: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err(format!("{name} may only be supplied once"))
    } else {
        Ok(())
    }
}
fn flag(slot: &mut bool, name: &str) -> Result<(), String> {
    if std::mem::replace(slot, true) {
        Err(format!("{name} may only be supplied once"))
    } else {
        Ok(())
    }
}
fn value(args: &mut impl Iterator<Item = OsString>, name: &str) -> Result<OsString, String> {
    args.next()
        .filter(|arg| {
            !arg.is_empty() && (name == "--editor-arg" || !arg.to_string_lossy().starts_with("--"))
        })
        .ok_or_else(|| match name {
            "--project" | "--document" => format!("{name} needs a path"),
            "--route" => "--route needs a URL".into(),
            _ => format!("{name} needs a value"),
        })
}
