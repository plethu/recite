use super::{AuthoringError, AuthoringKernel, SnapshotGeneration};
use crate::authoring::engine::{build_delta, build_documents, rebuild_analyses};
use crate::authoring::input_state::{
    changed_keys, effective_documents, unique_open, unique_saved, validate_overlay_versions,
};
use crate::authoring::{AnalysisDelta, AuthoringRequest, AuthoringSnapshot, WorkControl};
use std::sync::Arc;

impl AuthoringKernel {
    pub(super) fn prepare(
        &self,
        request: AuthoringRequest,
        control: &dyn WorkControl,
    ) -> Result<(Self, AnalysisDelta), AuthoringError> {
        control.checkpoint()?;
        let (expected_generation, saved_documents, open_documents, project_complete) =
            request.into_parts();
        if expected_generation != self.snapshot.generation() {
            return Err(AuthoringError::GenerationMismatch {
                expected: expected_generation,
                actual: self.snapshot.generation(),
            });
        }

        let saved = unique_saved(saved_documents)?;
        let open = unique_open(open_documents)?;
        validate_overlay_versions(&self.open, &open)?;
        let unchanged =
            saved == self.saved && open == self.open && project_complete == self.project_complete;
        let generation = if unchanged {
            self.snapshot.generation()
        } else {
            SnapshotGeneration(self.snapshot.generation().0.checked_add(1).ok_or(
                AuthoringError::GenerationExhausted {
                    current: self.snapshot.generation(),
                },
            )?)
        };

        let old_effective = effective_documents(&self.saved, &self.open);
        let new_effective = effective_documents(&saved, &open);
        let mut changed_inputs = changed_keys(&self.saved, &self.open, &saved, &open);
        if project_complete != self.project_complete {
            changed_inputs.extend(old_effective.keys().map(|key| (*key).clone()));
            changed_inputs.extend(new_effective.keys().map(|key| (*key).clone()));
        }
        let (analyses, project_changed) = rebuild_analyses(
            &self.analyses,
            &old_effective,
            &new_effective,
            self.schema.as_deref(),
            control,
        )?;
        if cfg!(test) && (!project_changed.is_empty() || project_complete != self.project_complete)
        {
            crate::authoring::engine::PROJECT_VALIDATION_COUNT
                .with(|count| count.set(count.get() + 1));
        }
        let mut project_index = Arc::clone(&self.project_index);
        let mut project_diagnostics = Arc::clone(&self.project_diagnostics);
        let project_changed =
            if project_changed.is_empty() && project_complete == self.project_complete {
                std::collections::BTreeSet::new()
            } else {
                Arc::make_mut(&mut project_index).update(
                    project_changed.into_iter().map(|key| {
                        let facts = analyses
                            .get(&key)
                            .map(|analysis| Arc::clone(&analysis.project_facts));
                        (key, facts)
                    }),
                    project_complete,
                    Arc::make_mut(&mut project_diagnostics),
                    control,
                )?
            };
        let documents = build_documents(
            &new_effective,
            &analyses,
            &project_diagnostics,
            &self.snapshot,
            &project_changed,
            control,
        )?;
        let (changed, removed) = build_delta(changed_inputs, &self.snapshot, &documents);
        let delta = AnalysisDelta::new(self.snapshot.generation(), generation, changed, removed);

        control.checkpoint()?;
        let candidate = Self {
            saved,
            open,
            analyses,
            project_complete,
            project_index,
            project_diagnostics,
            schema: self.schema.clone(),
            snapshot: AuthoringSnapshot::new(
                generation,
                documents,
                self.schema.clone(),
                project_complete,
            ),
        };
        Ok((candidate, delta))
    }
}
