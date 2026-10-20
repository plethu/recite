//! Validate the displayed command and spawn the producer away from the event loop.
use super::{
    Session,
    generation::{Generation, GenerationToken},
};
use crate::declarations::producer::Job;
use recite_config::ProducerRegistration;
use std::path::PathBuf;

pub struct GenerationStart {
    publication: GenerationToken,
    root: PathBuf,
    registration: ProducerRegistration,
}
pub struct GenerationStarted {
    publication: GenerationToken,
    result: Option<Result<Job, String>>,
}
impl Session {
    pub fn prepare_generation(&mut self) -> Result<GenerationStart, String> {
        self.require_idle_save()
            .map_err(|error| error.to_string())?;
        if self.busy() {
            return Err("Generation is already running.".into());
        }
        if self.dirty() {
            return Err("Save or discard the standalone source draft first.".into());
        }
        let registration = self
            .registration
            .as_ref()
            .map_err(Clone::clone)?
            .as_ref()
            .ok_or("No producer registered.")?
            .clone();
        let publication = GenerationToken::new();
        self.generation = Generation::Starting(publication.clone());
        Ok(GenerationStart {
            publication,
            root: self.root.clone(),
            registration,
        })
    }
    pub fn accept_generation_start(
        &mut self,
        receipt: &mut GenerationStarted,
    ) -> Result<(), String> {
        let Generation::Starting(publication) = &self.generation else {
            return Err("This generation is no longer awaiting its process.".into());
        };
        if !publication.matches(&receipt.publication) {
            return Err("This generation belongs to a different declaration session.".into());
        }
        let result = receipt
            .result
            .take()
            .ok_or("This generation receipt was already accepted.")?;
        if publication.cancelled()
            && let Ok(job) = &result
        {
            job.cancel();
        }
        self.generation = Generation::Idle;
        self.generation = Generation::Running(result?);
        Ok(())
    }
    pub fn start_generation(&mut self) -> Result<(), String> {
        let work = self.prepare_generation()?;
        self.accept_generation_start(&mut work.execute())
    }
}
impl GenerationStart {
    pub fn execute(self) -> GenerationStarted {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if self.publication.cancelled() {
                return Err("Generation cancelled. Previous schema retained.".into());
            }
            let current = Session::read_registration(&self.root, Some(self.registration.producer()))?;
            if current.as_ref() != Some(&self.registration) {
                return Err("Producer registration changed. Reload it and review the command before retrying.".into());
            }
            Job::start(self.root.clone(), self.registration.clone())
        })).unwrap_or_else(|_| Err("The schema producer could not be started.".into()));
        GenerationStarted {
            publication: self.publication,
            result: Some(result),
        }
    }
    pub fn not_started(self, error: String) -> GenerationStarted {
        GenerationStarted {
            publication: self.publication,
            result: Some(Err(error)),
        }
    }
}
