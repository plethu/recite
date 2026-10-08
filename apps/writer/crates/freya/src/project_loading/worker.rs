//! Owned cooperative load; cancellation never publishes a partially opened project.
use crate::project::{FileError, ProjectFiles};
use recite_compiler::authoring::{CancellationToken, Interrupted, WorkControl};
use recite_writer_model::Workbench;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
};

type LoadResult = Result<(ProjectFiles, Workbench), FileError>;

#[derive(Clone)]
struct Control {
    stop: CancellationToken,
    forwarded: Option<Arc<AtomicBool>>,
}

impl WorkControl for Control {
    fn checkpoint(&self) -> Result<(), Interrupted> {
        self.stop.checkpoint()?;
        if self
            .forwarded
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Acquire))
        {
            return Err(Interrupted);
        }
        Ok(())
    }
}

pub(super) struct Load {
    result: mpsc::Receiver<LoadResult>,
    worker: Option<JoinHandle<()>>,
    control: Control,
}

impl Load {
    pub(super) fn open(path: PathBuf, forwarded: Option<Arc<AtomicBool>>) -> std::io::Result<Self> {
        let control = Control {
            stop: CancellationToken::new(),
            forwarded,
        };
        let work = control.clone();
        let (sender, result) = mpsc::sync_channel(1);
        let worker = std::thread::Builder::new()
            .name("recite-project-open".into())
            .spawn(move || {
                let loaded =
                    ProjectFiles::open_with_control(&path, &work).and_then(|mut project| {
                        let workbench = project.workbench_with_control(&work)?;
                        work.checkpoint()?;
                        Ok((project, workbench))
                    });
                let _ = sender.send(loaded);
            })?;
        Ok(Self {
            result,
            worker: Some(worker),
            control,
        })
    }

    pub(super) fn cancel(&self) {
        self.control.stop.interrupt();
    }

    pub(super) fn cancelled(&self) -> bool {
        self.control.checkpoint().is_err()
    }

    pub(super) fn try_result(&self) -> Option<LoadResult> {
        match self.result.try_recv() {
            Ok(result) if self.cancelled() => {
                drop(result);
                Some(Err(Interrupted.into()))
            }
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(FileError::Io(
                std::io::Error::other("Project loading stopped unexpectedly"),
            ))),
        }
    }
}

impl Drop for Load {
    fn drop(&mut self) {
        self.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests;
