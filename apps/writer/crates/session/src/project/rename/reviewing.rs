use super::*;
impl ProjectFiles {
    pub fn review_rename(
        &mut self,
        model: &mut Workbench,
        block: &str,
        name: &str,
    ) -> Result<(), FileError> {
        self.require_idle_save()?;
        self.require_navigation_ready()?;
        if model.has_draft() || self.retained.values().any(|s| s.model.has_draft()) {
            return Err(FileError::UnsavedDocument);
        }
        self.refresh(model)?;
        let plan = model
            .document()
            .plan_project_rename(block, name)
            .map_err(recite_writer_model::WorkbenchError::from)?;
        let mut baseline = self
            .paths
            .iter()
            .map(|p| super::read_regular(p).map(|source| (p.clone(), source)))
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let manifest_path = self.root.join("recite.project.toml");
        baseline.insert(manifest_path.clone(), super::read_regular(&manifest_path)?);
        let manifest_before = self.manifest.text().to_owned();
        let manifest_after =
            recite_writer_model::rename_manifest_source(&manifest_before, block, name)
                .map_err(recite_writer_model::WorkbenchError::from)?;
        self.rename_review = Some(RenameReview {
            manifest_before,
            manifest_after,
            plan,
            baseline,
            block: block.into(),
            name: name.into(),
        });
        Ok(())
    }
}
