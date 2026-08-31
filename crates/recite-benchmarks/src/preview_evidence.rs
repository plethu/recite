use recite_runtime::{PreviewSession, PreviewSnapshot};

use crate::preview::{PreviewEvidenceReport, PreviewProject, PreviewRestoreParity};
use crate::{BenchmarkResult, error};

impl PreviewProject {
    /// Captures the exact encoded snapshot size and deterministic retained
    /// trace/transcript shape at the supplied preview boundary.
    pub fn retention_report(
        &self,
        preview: &PreviewSession<'_>,
    ) -> BenchmarkResult<crate::preview::PreviewRetentionReport> {
        crate::preview_retention::build_report(self.fixture_label(), preview, preview)
    }

    pub fn retained_trace_shape(
        &self,
        preview: &PreviewSession<'_>,
    ) -> crate::preview::PreviewTraceShape {
        crate::preview_shape::trace_shape(preview.trace())
    }

    pub fn evidence_report(&self) -> BenchmarkResult<PreviewEvidenceReport> {
        let mut traversal_preview = self.start()?;
        let traversal = self.traversal_summary(&mut traversal_preview)?;
        let snapshot_preview = self.after_first_choice()?;
        let retention = crate::preview_retention::build_report(
            self.fixture_label(),
            &snapshot_preview,
            &traversal_preview,
        )?;
        Ok(PreviewEvidenceReport {
            fixture: self.fixture_label(),
            traversal,
            retention,
            restore: self.restore_parity()?,
        })
    }

    /// Restores a prompt snapshot into a fresh session and compares all future
    /// externally visible events through the end of the scene.
    pub fn restore_parity(&self) -> BenchmarkResult<PreviewRestoreParity> {
        let mut original = self.at_first_prompt()?;
        let snapshot = original.snapshot().map_err(preview_error)?;
        let encoded = snapshot.encode().map_err(preview_error)?;
        let decoded = PreviewSnapshot::decode(&encoded).map_err(preview_error)?;
        let mut restored = self.start()?;
        restored.restore(decoded).map_err(preview_error)?;

        let original_events = self.collect_to_end(&mut original)?;
        let restored_events = self.collect_to_end(&mut restored)?;
        Ok(PreviewRestoreParity {
            events_match: original_events == restored_events,
            original_event_count: original_events.len(),
            restored_event_count: restored_events.len(),
        })
    }
}

fn preview_error(preview: recite_runtime::PreviewError) -> crate::BenchmarkError {
    error(format!("preview operation failed: {preview}"))
}
