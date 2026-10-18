//! Keep reference checks separate so unchanged declaration diagnostics can survive an edit.
use recite_core::{Diagnostic, DocumentKey};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) type ProjectDiagnostics = BTreeMap<DocumentKey, ProjectReports>;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ProjectReports {
    pub(super) structural: Arc<Vec<Diagnostic>>,
    pub(super) references: Arc<Vec<Diagnostic>>,
}

impl ProjectReports {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.structural.iter().chain(self.references.iter())
    }

    pub(super) fn iter_mut(&mut self) -> impl Iterator<Item = &mut Diagnostic> {
        Arc::make_mut(&mut self.structural)
            .iter_mut()
            .chain(Arc::make_mut(&mut self.references))
    }
}
