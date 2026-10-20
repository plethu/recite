use std::{ffi::OsString, path::PathBuf};

pub struct Startup {
    pub project: PathBuf,
    pub document: Option<PathBuf>,
    pub editor: Option<OsString>,
    pub editor_args: Vec<OsString>,
}

pub use recite_writer_session::startup::{HELP, LaunchOptions};

impl Startup {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let options = LaunchOptions::parse(args)?;
        let recite_writer_session::startup::Startup::Writer {
            project: Some(project),
            ..
        } = options.intent
        else {
            return Err("Choose a project with --project PATH".into());
        };
        Ok(Self {
            project,
            document: options.document,
            editor: options.editor.as_ref().map(|editor| editor.program.clone()),
            editor_args: options.editor.map_or_else(Vec::new, |editor| editor.args),
        })
    }
}
