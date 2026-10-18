//! Destination analysis is prepared without moving the active or retained session owners.
use super::super::{FileError, ProjectFiles, ReadyCheckpoint, read_regular};
use super::{Retained, overlay};
use crate::recovery::RecoveryStore;
use recite_compiler::authoring::WorkControl;
use recite_writer_model::{
    CompletionSnapshot, NavigationRequest, PreparedNavigation, ProjectContext, View, Workbench,
    WorkbenchError,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

enum Opening {
    Retained(NavigationRequest),
    Cold {
        sibling: CompletionSnapshot,
        key: recite_core::DocumentKey,
        initial: Option<View>,
    },
}
pub(crate) struct SwitchRequest {
    from: PathBuf,
    origin: CompletionSnapshot,
    path: PathBuf,
    context: ProjectContext,
    opening: Opening,
}
enum Destination {
    Retained(PreparedNavigation),
    Cold(Option<Retained>),
}
pub(crate) struct PreparedSwitch {
    from: PathBuf,
    origin: CompletionSnapshot,
    path: PathBuf,
    destination: Destination,
}
impl ProjectFiles {
    pub(crate) fn switch_request(
        &self,
        current: &Workbench,
        path: &Path,
        initial: Option<View>,
    ) -> Result<SwitchRequest, FileError> {
        if path == self.current || !self.paths.iter().any(|candidate| candidate == path) {
            return Err(FileError::Selection);
        }
        let mut context = self.retained_context(self.context.clone());
        overlay(&mut context, current);
        let opening = match self.retained.get(path) {
            Some(session) => Opening::Retained(session.model.navigation_request()),
            None => Opening::Cold {
                sibling: current.document().completion_snapshot(),
                key: recite_core::DocumentKey::new(
                    self.names.get(path).ok_or(FileError::Selection)?,
                )
                .map_err(recite_writer_model::EditError::from)
                .map_err(WorkbenchError::from)?,
                initial,
            },
        };
        Ok(SwitchRequest {
            from: self.current.clone(),
            origin: current.document().completion_snapshot(),
            path: path.to_owned(),
            context,
            opening,
        })
    }
    pub(crate) fn accept_switch(
        &mut self,
        current: &mut Workbench,
        prepared: &mut PreparedSwitch,
        checkpoint: &ReadyCheckpoint,
    ) -> Result<(), FileError> {
        if !prepared.is_current(self, current, checkpoint) {
            return Err(WorkbenchError::from(recite_writer_model::EditError::Stale).into());
        }
        let mut next = match &mut prepared.destination {
            Destination::Retained(selection) => {
                self.retained
                    .get_mut(&prepared.path)
                    .ok_or(FileError::Selection)?
                    .model
                    .accept_navigation(selection)?;
                self.retained
                    .remove(&prepared.path)
                    .ok_or(FileError::Selection)?
            }
            Destination::Cold(next) => next.take().ok_or(FileError::Selection)?,
        };
        std::mem::swap(current, &mut next.model);
        std::mem::swap(&mut self.saved, &mut next.baseline);
        std::mem::swap(&mut self.recovery, &mut next.recovery);
        let previous = std::mem::replace(&mut self.current, prepared.path.clone());
        self.retained.insert(previous, next);
        Ok(())
    }
}
impl SwitchRequest {
    pub(crate) fn prepare(
        self,
        control: &dyn WorkControl,
        select: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>,
    ) -> Result<PreparedSwitch, FileError> {
        control.checkpoint()?;
        let destination = match self.opening {
            Opening::Retained(request) => {
                let mut prepared = request.prepare(Some(self.context), control)?;
                select(prepared.model_mut())?;
                Destination::Retained(prepared)
            }
            Opening::Cold {
                sibling,
                key,
                initial,
            } => {
                let mut baseline: Arc<str> = read_regular(&self.path)?.into();
                let recovery = RecoveryStore::open(&self.path)?;
                let source = recovery
                    .snapshot()
                    .map_or(baseline.as_ref(), |r| r.draft.source());
                let document = sibling
                    .open_sibling_with_control(key, source, self.context, control)
                    .map_err(WorkbenchError::from)?;
                let view = recovery
                    .snapshot()
                    .map(|r| r.draft.view().clone())
                    .or(initial);
                let mut model = Workbench::from_document(document, view)?;
                if let Some(snapshot) = recovery.snapshot() {
                    snapshot.draft.restore(&mut model)?;
                    baseline = snapshot.baseline.clone();
                }
                select(&mut model)?;
                Destination::Cold(Some(Retained {
                    model,
                    baseline,
                    recovery,
                }))
            }
        };
        control.checkpoint()?;
        Ok(PreparedSwitch {
            from: self.from,
            origin: self.origin,
            path: self.path,
            destination,
        })
    }
}

impl PreparedSwitch {
    pub(crate) fn refresh_rules(
        &self,
        rules: &mut recite_writer_model::ReplyRules,
        model: &Workbench,
    ) -> bool {
        match &self.destination {
            Destination::Retained(prepared) => prepared.refresh_rules(rules, model.document()),
            Destination::Cold(_) => false,
        }
    }
    pub(crate) fn is_current(
        &self,
        files: &ProjectFiles,
        current: &Workbench,
        checkpoint: &ReadyCheckpoint,
    ) -> bool {
        checkpoint.is_current(files, current)
            && self.from == files.current
            && self.origin.is_current(current.document())
            && files.paths.contains(&self.path)
            && match &self.destination {
                Destination::Retained(prepared) => files
                    .retained
                    .get(&self.path)
                    .is_some_and(|target| prepared.is_current(&target.model)),
                Destination::Cold(next) => {
                    next.is_some() && !files.retained.contains_key(&self.path)
                }
            }
    }
}
