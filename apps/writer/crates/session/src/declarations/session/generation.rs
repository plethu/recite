//! A process completion is still busy until checked output publication is reconciled.
use super::Session;
use crate::declarations::producer::{Generated, Job};
use crate::project::save::replace_checked;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub(super) enum Generation {
    Idle,
    Starting(GenerationToken),
    Running(Job),
    Publishing(GenerationToken),
}
impl Generation {
    pub fn cancel(&self) {
        match self {
            Self::Idle => (),
            Self::Running(job) => job.cancel(),
            Self::Starting(publication) | Self::Publishing(publication) => publication.cancel(),
        }
    }
}
impl Drop for Generation {
    fn drop(&mut self) {
        self.cancel();
    }
}
/// Identity belongs to this publication, independently of the visible screen or draft.
#[derive(Clone)]
pub(super) struct GenerationToken(Arc<AtomicBool>);
impl GenerationToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
    pub fn matches(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// Immutable capture for a worker. The live Session retains its recovery lease.
pub struct GenerationWork {
    publication: GenerationToken,
    output: PathBuf,
    baseline: String,
    generated: Generated,
}
/// A terminal receipt is consumed once, only by the originating Session.
pub struct GenerationReceipt {
    publication: GenerationToken,
    result: Option<Result<Generated, String>>,
    retired: Option<Generated>,
}
impl Session {
    /// Polling never publishes to disk. Successful process output starts publication.
    pub fn poll_generation(&mut self) -> Option<Result<GenerationWork, String>> {
        let Generation::Running(job) = &self.generation else {
            return None;
        };
        let result = job.poll()?;
        self.generation = Generation::Idle;
        Some(result.map(|generated| self.begin_publication(generated)))
    }
    fn begin_publication(&mut self, generated: Generated) -> GenerationWork {
        let publication = GenerationToken::new();
        self.generation = Generation::Publishing(publication.clone());
        GenerationWork {
            publication,
            output: self.output.clone(),
            baseline: self.output_baseline.clone(),
            generated,
        }
    }
    pub fn accept_generation(&mut self, receipt: &mut GenerationReceipt) -> Result<(), String> {
        let Generation::Publishing(publication) = &self.generation else {
            return Err("This generation is no longer awaiting publication.".into());
        };
        if !publication.matches(&receipt.publication) {
            return Err("This generation belongs to a different declaration session.".into());
        }
        let result = receipt
            .result
            .take()
            .ok_or("This generation receipt was already accepted.")?;
        self.generation = Generation::Idle;
        let mut generated = result?;
        // Cancellation after the write cannot undo a successful publication.
        std::mem::swap(&mut self.output_baseline, &mut generated.text);
        std::mem::swap(&mut self.schema, &mut generated.schema);
        receipt.retired = Some(generated);
        Ok(())
    }
    /// Compatibility path for hosts that already perform synchronous project refreshes.
    pub fn poll_generation_blocking(&mut self) -> Option<Result<(), String>> {
        Some(match self.poll_generation()? {
            Ok(work) => self.accept_generation(&mut work.execute()),
            Err(error) => Err(error),
        })
    }
}
impl GenerationWork {
    pub fn execute(self) -> GenerationReceipt {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if self.publication.cancelled() {
                return Err("Generation cancelled. Previous schema retained.".into());
            }
            replace_checked(&self.output, &self.baseline, &self.generated.text)
                .map_err(|error| error.to_string())?;
            Ok(())
        }))
        .unwrap_or_else(|_| Err("Schema publication stopped unexpectedly.".into()));
        GenerationReceipt {
            publication: self.publication,
            result: Some(result.map(|()| self.generated)),
            retired: None,
        }
    }
    /// Failed dispatch must also retire the live publication state.
    pub fn not_started(self, error: String) -> GenerationReceipt {
        GenerationReceipt {
            publication: self.publication,
            result: Some(Err(error)),
            retired: Some(self.generated),
        }
    }
}
impl GenerationReceipt {
    pub fn schema(&self) -> Option<&recite_core::schema::ProjectSchema> {
        self.result
            .as_ref()?
            .as_ref()
            .ok()
            .map(|generated| &generated.schema)
    }
}

#[cfg(test)]
mod tests;
