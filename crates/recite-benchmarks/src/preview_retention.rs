use recite_runtime::{PreviewSession, PreviewSnapshot};

use crate::preview_shape::{PreviewTraceShape, trace_shape};
use crate::{BenchmarkResult, error};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewRetentionReport {
    pub fixture: &'static str,
    pub snapshot: PreviewSnapshotShape,
    pub trace: PreviewTraceShape,
    pub transcript_events: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewSnapshotShape {
    pub encoded_bytes: usize,
    pub selected_choice_count: usize,
    pub deferred_effect_count: usize,
}

pub(crate) fn build_report(
    fixture: &'static str,
    snapshot_preview: &PreviewSession<'_>,
    trace_preview: &PreviewSession<'_>,
) -> BenchmarkResult<PreviewRetentionReport> {
    let snapshot = snapshot_preview.snapshot().map_err(preview_error)?;
    let snapshot_encoded_bytes = snapshot.encode().map_err(preview_error)?.len();
    Ok(PreviewRetentionReport {
        fixture,
        snapshot: snapshot_shape(&snapshot, snapshot_encoded_bytes),
        trace: trace_shape(trace_preview.trace()),
        transcript_events: trace_preview.transcript().events().len(),
    })
}

fn preview_error(preview: recite_runtime::PreviewError) -> crate::BenchmarkError {
    error(format!("preview operation failed: {preview}"))
}

fn snapshot_shape(snapshot: &PreviewSnapshot, encoded_bytes: usize) -> PreviewSnapshotShape {
    PreviewSnapshotShape {
        encoded_bytes,
        selected_choice_count: snapshot.state().selected_choice_history().len(),
        deferred_effect_count: snapshot.state().deferred_effects().len(),
    }
}
