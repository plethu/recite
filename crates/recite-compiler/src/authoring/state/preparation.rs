use super::{AuthoringError, AuthoringKernel, SnapshotGeneration};
use crate::authoring::engine::{self, build_delta, build_documents, rebuild_analyses};
use crate::authoring::input::RequestParts;
use crate::authoring::input_state::{
    changed_keys, effective_documents, unique_open, unique_saved, validate_overlay_versions,
};
use crate::authoring::{
    AnalysisDelta, AuthoringRequest, AuthoringSnapshot, CatalogSnapshot, Interrupted, WorkControl,
};
use std::sync::Arc;

impl AuthoringKernel {
    pub(super) fn prepare(
        &self,
        request: AuthoringRequest,
        control: &dyn WorkControl,
    ) -> Result<(Self, AnalysisDelta), AuthoringError> {
        control.checkpoint()?;
        let RequestParts {
            expected_generation,
            saved_documents,
            open_documents,
            saved_catalogs,
            open_catalogs,
            project_complete,
        } = request.into_parts();
        if expected_generation != self.snapshot.generation() {
            return Err(AuthoringError::GenerationMismatch {
                expected: expected_generation,
                actual: self.snapshot.generation(),
            });
        }

        let saved = unique_saved(saved_documents)?;
        let open = unique_open(open_documents)?;
        validate_overlay_versions(&self.open, &open)?;
        let saved_catalogs = unique_saved(saved_catalogs)?;
        let open_catalogs = unique_open(open_catalogs)?;
        validate_overlay_versions(&self.open_catalogs, &open_catalogs)?;
        if let Some(key) = saved_catalogs
            .keys()
            .chain(open_catalogs.keys())
            .find(|key| saved.contains_key(*key) || open.contains_key(*key))
        {
            return Err(AuthoringError::CatalogKeyConflict { key: key.clone() });
        }
        let unchanged = saved == self.saved
            && open == self.open
            && saved_catalogs == self.saved_catalogs
            && open_catalogs == self.open_catalogs
            && project_complete == self.project_complete;
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
            engine::PROJECT_VALIDATION_COUNT.with(|count| count.set(count.get() + 1));
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
        let catalogs = build_catalogs(&self.snapshot, &saved_catalogs, &open_catalogs, control)?;
        let (changed, removed) = build_delta(changed_inputs, &self.snapshot, &documents);
        let delta = AnalysisDelta::new(self.snapshot.generation(), generation, changed, removed);

        control.checkpoint()?;
        let candidate = Self {
            saved,
            open,
            saved_catalogs,
            open_catalogs,
            analyses,
            project_complete,
            project_index,
            project_diagnostics,
            schema: self.schema.clone(),
            snapshot: AuthoringSnapshot::new(
                generation,
                documents,
                catalogs,
                self.schema.clone(),
                project_complete,
            ),
        };
        Ok((candidate, delta))
    }
}

/// Catalogue snapshots in key order, reusing unchanged line indexes.
fn build_catalogs(
    previous: &AuthoringSnapshot,
    saved: &std::collections::BTreeMap<recite_core::DocumentKey, crate::authoring::SavedDocument>,
    open: &std::collections::BTreeMap<recite_core::DocumentKey, crate::authoring::OpenDocument>,
    control: &dyn WorkControl,
) -> Result<Vec<CatalogSnapshot>, Interrupted> {
    let effective = effective_documents(saved, open);
    let mut catalogs = Vec::with_capacity(effective.len());
    for (key, document) in effective {
        control.checkpoint()?;
        let reused = previous
            .catalog(key)
            .filter(|catalog| catalog.describes(document.layer, document.version, document.text));
        catalogs.push(reused.cloned().unwrap_or_else(|| {
            CatalogSnapshot::new(key.clone(), document.layer, document.version, document.text)
        }));
    }
    Ok(catalogs)
}
