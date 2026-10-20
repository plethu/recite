//! A process completion is still busy until checked output publication is reconciled.
use super::{
    SchemaOutput, Session,
    activity::{Activity, ActivityToken},
};
use crate::declarations::producer::Generated;
use crate::project::save::replace_checked;
use std::{path::PathBuf, sync::Arc};

/// Immutable capture for a worker. The live Session retains its recovery lease.
pub struct GenerationWork {
    publication: ActivityToken,
    output: PathBuf,
    baseline: Arc<SchemaOutput>,
    generated: Generated,
}
/// A terminal receipt is consumed once, only by the originating Session.
pub struct GenerationReceipt {
    publication: ActivityToken,
    result: Option<Result<Generated, String>>,
    retired: Option<Arc<SchemaOutput>>,
}
impl Session {
    /// Polling never publishes to disk. Successful process output starts publication.
    pub fn poll_generation(&mut self) -> Option<Result<GenerationWork, String>> {
        let Activity::Generating(job) = &self.activity else {
            return None;
        };
        let result = job.poll()?;
        self.activity = Activity::Idle;
        Some(result.map(|generated| self.begin_publication(generated)))
    }
    fn begin_publication(&mut self, generated: Generated) -> GenerationWork {
        let publication = ActivityToken::new();
        self.activity = Activity::PublishingGeneration(publication.clone());
        GenerationWork {
            publication,
            output: self.output.clone(),
            baseline: self.schema_output.clone(),
            generated,
        }
    }
    pub fn accept_generation(&mut self, receipt: &mut GenerationReceipt) -> Result<(), String> {
        let Activity::PublishingGeneration(publication) = &self.activity else {
            return Err("This generation is no longer awaiting publication.".into());
        };
        if !publication.matches(&receipt.publication) {
            return Err("This generation belongs to a different declaration session.".into());
        }
        let result = receipt
            .result
            .take()
            .ok_or("This generation receipt was already accepted.")?;
        self.activity = Activity::Idle;
        let generated = result?;
        // Cancellation after the write cannot undo a successful publication.
        receipt.retired = Some(std::mem::replace(
            &mut self.schema_output,
            Arc::new(SchemaOutput::Ready(generated)),
        ));
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
            replace_checked(&self.output, self.baseline.baseline(), &self.generated.text)
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
            retired: Some(Arc::new(SchemaOutput::Ready(self.generated))),
        }
    }
}
impl GenerationReceipt {
    pub fn schema(&self) -> Option<&recite_core::schema::ProjectSchema> {
        self.result
            .as_ref()?
            .as_ref()
            .ok()
            .map(|generated| generated.schema.as_ref())
    }
}

#[cfg(test)]
mod tests;
