//! Capture saved input cheaply; observation owns no model, compiler graph or recovery lease.
use super::{FileError, ProjectFiles, read_regular};
use crate::external::Affected;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Weak},
};

pub struct ExternalCheck {
    path: PathBuf,
    baseline: Arc<str>,
}
pub struct ExternalObservation {
    path: PathBuf,
    baseline: Weak<str>,
    changed: Result<bool, FileError>,
}
impl ExternalCheck {
    pub fn observe(self) -> ExternalObservation {
        let changed = read_regular(&self.path).map(|disk| disk != self.baseline.as_ref());
        ExternalObservation {
            path: self.path,
            baseline: Arc::downgrade(&self.baseline),
            changed,
        }
    }
}
impl ExternalObservation {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn changed(&self) -> bool {
        self.changed.as_ref().map_or(true, |changed| *changed)
    }
    pub fn is_current(&self, files: &ProjectFiles) -> bool {
        files
            .saved_baseline(&self.path)
            .is_some_and(|baseline| Weak::ptr_eq(&self.baseline, &Arc::downgrade(baseline)))
    }
}
impl ProjectFiles {
    fn saved_baseline(&self, path: &Path) -> Option<&Arc<str>> {
        if path == self.current {
            Some(&self.saved)
        } else {
            self.retained.get(path).map(|session| &session.baseline)
        }
    }
    pub fn external_check(&self, path: &Path) -> Option<ExternalCheck> {
        self.saved_baseline(path).map(|baseline| ExternalCheck {
            path: path.to_owned(),
            baseline: baseline.clone(),
        })
    }
    pub fn external_checks(&self, affected: &Affected) -> Vec<ExternalCheck> {
        match affected {
            Affected::Paths(paths) => paths
                .iter()
                .filter_map(|path| self.external_check(path))
                .collect(),
            Affected::AllOpen => std::iter::once(&self.current)
                .chain(self.retained.keys())
                .filter_map(|path| self.external_check(path))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests;
