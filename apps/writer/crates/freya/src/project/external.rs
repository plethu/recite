//! An external comparison is bound to one file and one inspected disk version.
use super::{FileError, ProjectFiles, read_regular};
use recite_writer_model::{View, Workbench};
use std::path::PathBuf;
#[derive(Clone)]
pub(crate) struct ExternalComparison {
    pub path: PathBuf,
    pub draft: String,
    pub disk: String,
}
impl ProjectFiles {
    pub fn compare_external(&mut self, model: &mut Workbench) -> Result<(), FileError> {
        if model.view() != &View::Source {
            model.select(View::Source)?;
        }
        self.external = Some(ExternalComparison {
            path: self.current.clone(),
            draft: model.draft().into(),
            disk: read_regular(&self.current)?,
        });
        Ok(())
    }
    pub fn resolve_external(
        &mut self,
        model: &mut Workbench,
        keep: bool,
    ) -> Result<PathBuf, FileError> {
        let comparison = self.external.as_ref().ok_or(FileError::Selection)?;
        if comparison.path != self.current
            || model.view() != &View::Source
            || model.draft() != comparison.draft
            || read_regular(&comparison.path)? != comparison.disk
        {
            return Err(FileError::Conflict);
        }
        let next = if keep {
            comparison.draft.clone()
        } else {
            comparison.disk.clone()
        };
        let disk = comparison.disk.clone();
        let copy = self.export(model)?;
        model.set_draft(next);
        if !keep {
            model.apply()?;
        }
        self.saved = disk.into();
        self.update_saved_context()?;
        self.checkpoint(model)?;
        Ok(copy)
    }
    pub fn externally_changed(&self, path: &std::path::Path) -> bool {
        let baseline = if path == self.current {
            Some(self.saved.as_ref())
        } else {
            self.retained.get(path).map(|s| s.baseline.as_ref())
        };
        baseline.is_some_and(|baseline| read_regular(path).map_or(true, |disk| disk != baseline))
    }
}

#[cfg(test)]
mod tests;
