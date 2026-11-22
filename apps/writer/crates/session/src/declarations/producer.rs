//! Explicit producer processes write staged output; only validated results reach publication.
use recite_config::{PRODUCER_DEADLINE, ProducerCommand, ProducerRegistration, ProducerRunError};
use recite_core::schema::ProjectSchema;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

pub struct Generated {
    pub(super) text: String,
    pub(super) schema: Arc<ProjectSchema>,
}
pub struct Job {
    cancel: Arc<AtomicBool>,
    result: mpsc::Receiver<Result<Generated, String>>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
impl Job {
    pub fn start(root: PathBuf, registration: ProducerRegistration) -> Result<Self, String> {
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let (send, result) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("recite-schema-producer".into())
            .spawn(move || {
                let _ = send.send(generate(&root, &registration, &flag));
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { cancel, result })
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
    pub fn poll(&self) -> Option<Result<Generated, String>> {
        match self.result.try_recv() {
            Ok(value) => Some(if self.cancel.load(Ordering::Relaxed) {
                Err("Generation cancelled. Previous schema retained.".into())
            } else {
                value
            }),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(
                "The schema producer worker stopped unexpectedly.".into(),
            )),
        }
    }
}
/// The configured command for an editor or producer, as `recite_config` builds it for every client.
pub fn command(
    root: &Path,
    spec: &ProducerCommand,
    replacements: &[(&str, String)],
) -> Result<Command, String> {
    recite_config::producer_command(root, spec, replacements).map_err(|error| match error {
        ProducerRunError::DirectoryOutsideProject => {
            "The producer working directory resolves outside this project.".into()
        }
        other => other.to_string(),
    })
}

fn generate(
    root: &Path,
    registration: &ProducerRegistration,
    cancel: &AtomicBool,
) -> Result<Generated, String> {
    match recite_config::generate_schema(root, registration, cancel, PRODUCER_DEADLINE) {
        Ok(generated) => Ok(Generated {
            text: generated.text,
            schema: generated.schema.into(),
        }),
        Err(ProducerRunError::Start { program, source }) => {
            Err(format!("Could not start {program}: {source}"))
        }
        Err(ProducerRunError::Stopped) => Err(
            "Generation cancelled or exceeded two minutes. The previous schema remains available."
                .into(),
        ),
        Err(ProducerRunError::Failed { status, log }) => Err(format!(
            "Producer exited with {status}. Previous schema retained.\n{log}"
        )),
        Err(ProducerRunError::Output(error)) => Err(format!(
            "Producer did not supply a regular schema output: {error}"
        )),
        Err(ProducerRunError::Invalid(diagnostics)) => {
            Err(crate::diagnostic_messages(&diagnostics))
        }
        Err(ProducerRunError::WrongProducer) => Err(
            "Generated schema belongs to a different producer. Previous schema retained.".into(),
        ),
        Err(other) => Err(other.to_string()),
    }
}
#[cfg(test)]
mod tests;
