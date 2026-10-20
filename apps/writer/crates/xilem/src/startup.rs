use std::{ffi::OsString, path::PathBuf};

pub struct Startup {
    pub project: PathBuf,
    pub document: Option<PathBuf>,
    pub editor: Option<OsString>,
    pub editor_args: Vec<OsString>,
}

pub const HELP: &str = "Recite Writer (Xilem migration)\n\n--project PATH         Open a Recite project\n--document PATH        Choose a project-relative source file\n--editor PROGRAM       Preferred editor executable (no shell)\n--editor-arg ARG       Repeat for editor arguments; the file path is appended\n--help                Show this help";

impl Startup {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let mut project = None;
        let mut document = None;
        let mut editor = None;
        let mut editor_args = Vec::new();
        while let Some(arg) = args.next() {
            let value = args
                .next()
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("{} needs a value", arg.to_string_lossy()))?;
            match arg.to_str() {
                Some("--project") if project.is_none() => project = Some(PathBuf::from(value)),
                Some("--document") if document.is_none() => document = Some(PathBuf::from(value)),
                Some("--editor") if editor.is_none() => editor = Some(value),
                Some("--editor-arg") => editor_args.push(value),
                _ => {
                    return Err(format!(
                        "Unknown or repeated argument: {}",
                        arg.to_string_lossy()
                    ));
                }
            }
        }
        if editor.is_none() && !editor_args.is_empty() {
            return Err("--editor-arg requires --editor".into());
        }
        Ok(Self {
            project: project.ok_or("Choose a project with --project PATH")?,
            document,
            editor,
            editor_args,
        })
    }
}
