//! The OS owns preferred-application registration. Paths are passed as argv.
use crate::{
    editing::Writer,
    messages::{MsgId, text},
};
use std::{path::PathBuf, sync::mpsc};
pub(crate) struct Handoff {
    result: mpsc::Receiver<Result<(), String>>,
}
impl Handoff {
    fn start(path: PathBuf) -> Result<Self, String> {
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
pub(crate) fn open_editor(mut writer: Writer) {
    if writer
        .files
        .peek()
        .as_ref()
        .is_some_and(|p| p.handoff.is_some())
    {
        return;
    }
    if let Err(e) = writer.buffers.save(writer.files) {
        writer.message.error(e);
        return;
    }
    let mut files = writer.files.write();
    let Some(project) = files.as_mut() else {
        return;
    };
    match Handoff::start(project.current.clone()) {
        Ok(job) => {
            project.handoff = Some(job);
            writer.message.info(text(MsgId::WriterExternalOpening));
        }
        Err(e) => writer.message.error(e),
    }
}
