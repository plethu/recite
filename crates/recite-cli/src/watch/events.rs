use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::Event;

use super::PROJECT_MANIFEST_FILE;
use super::inputs::{is_generated_output_path, is_project_recite_source};
use crate::error::CliError;
use crate::i18n::{Messages, MsgId};

const DEBOUNCE: Duration = Duration::from_millis(250);

// Instant::now is intentional here: this is CLI file-watcher debounce logic,
// not deterministic dialogue runtime code. The absolute deadline is tracked so
// irrelevant events (generated output writes) consume the window without
// resetting it; do not simplify to a per-loop recv_timeout(DEBOUNCE).
#[allow(clippy::disallowed_methods)]
pub(super) fn drain_debounce(
    receiver: &mpsc::Receiver<notify::Result<Event>>,
    state: &WatchState,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let mut deadline = Instant::now() + DEBOUNCE;
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Ok(());
        }

        match receiver.recv_timeout(deadline - now) {
            Ok(Ok(event)) => {
                // Events are wakeups only. Relevant wakeups extend the fixed
                // debounce; generated asset writes are intentionally ignored.
                if state.is_relevant_event(&event) {
                    deadline = Instant::now() + DEBOUNCE;
                }
            }
            Ok(Err(error)) => {
                writeln!(
                    stderr,
                    "{}",
                    messages.format(MsgId::WatchEventError, [("error", error.to_string())])
                )?;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => return Ok(()),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(CliError::Watch {
                    message: "watcher event channel closed".to_owned(),
                });
            }
        }
    }
}

pub(super) fn watch_error(error: notify::Error) -> CliError {
    CliError::Watch {
        message: format!("failed to start watcher: {error}"),
    }
}

#[derive(Debug)]
pub(super) struct WatchState {
    pub(super) project_root: PathBuf,
    pub(super) schema_path: Option<PathBuf>,
}

impl WatchState {
    pub(super) fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            schema_path: None,
        }
    }

    pub(super) fn manifest_path(&self) -> PathBuf {
        self.project_root.join(PROJECT_MANIFEST_FILE)
    }

    pub(super) fn is_relevant_event(&self, event: &Event) -> bool {
        event.paths.is_empty() || event.paths.iter().any(|path| self.is_relevant_path(path))
    }

    pub(super) fn is_relevant_path(&self, path: &Path) -> bool {
        let path = if path.is_absolute() {
            path.to_owned()
        } else {
            self.project_root.join(path)
        };

        if is_generated_output_path(&path) {
            return false;
        }
        if path == self.manifest_path() {
            return true;
        }
        if self
            .schema_path
            .as_ref()
            .is_some_and(|schema| schema == &path)
        {
            return true;
        }
        is_project_recite_source(&self.project_root, &path)
    }
}
