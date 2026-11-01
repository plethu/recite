//! Independently owned external application processes.
use std::{path::PathBuf, sync::mpsc};
pub struct Handoff {
    result: mpsc::Receiver<Result<(), String>>,
}
impl Handoff {
    pub fn configured(
        path: PathBuf,
        editor: Option<&crate::startup::EditorCommand>,
    ) -> Result<Self, String> {
        match editor {
            Some(editor) => {
                let mut command = std::process::Command::new(&editor.program);
                command.args(&editor.args).arg(path);
                Self::command(command)
            }
            None => Self::start(path),
        }
    }
    pub fn start(path: PathBuf) -> Result<Self, String> {
        let program = if cfg!(target_os = "macos") {
            "open"
        } else if cfg!(target_os = "windows") {
            "explorer.exe"
        } else {
            "xdg-open"
        };
        let mut command = std::process::Command::new(program);
        command.arg(path);
        Self::command(command)
    }
    pub fn command(mut command: std::process::Command) -> Result<Self, String> {
        let (send, result) = mpsc::sync_channel(1);
        std::thread::Builder::new().name("recite-external-editor".into()).spawn(move || {
            let status = command.status().map_err(|e| e.to_string()).and_then(|status| {
                if status.success() { Ok(()) } else { Err(format!("The external editor exited with {status}. Check the configured editor command.")) }
            });
            let _ = send.send(status);
        }).map_err(|e| e.to_string())?;
        Ok(Self { result })
    }
    pub fn poll(&self) -> Option<Result<(), String>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(
                "The external application launcher stopped unexpectedly.".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests;
