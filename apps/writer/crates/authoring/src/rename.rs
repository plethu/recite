//! Reviewable project rename patches come exclusively from the compiler planner.
use crate::{Document, EditError};
use recite_core::{DocumentKey, SourcePosition};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameChange {
    pub document: DocumentKey,
    pub before: String,
    pub after: String,
}
#[derive(Clone, Debug)]
pub struct ProjectRename {
    pub changes: Vec<RenameChange>,
}
impl Document {
    pub fn plan_project_rename(&self, block: &str, name: &str) -> Result<ProjectRename, EditError> {
        let snapshot = self.kernel().snapshot();
        let declaration = snapshot
            .document(self.key())
            .ok_or(EditError::Position)?
            .summary()
            .blocks()
            .iter()
            .find(|b| b.id().as_str() == block)
            .ok_or(EditError::Destination)?;
        let plan = snapshot.plan_rename_block(
            self.key(),
            SourcePosition::new(declaration.span().start.line(), 4)?,
            name,
        )?;
        let changes = plan
            .apply(snapshot)?
            .into_iter()
            .map(|(document, after)| {
                let before = snapshot
                    .document(&document)
                    .ok_or(EditError::Position)?
                    .source_text()
                    .to_owned();
                Ok(RenameChange {
                    document,
                    before,
                    after,
                })
            })
            .collect::<Result<_, EditError>>()?;
        Ok(ProjectRename { changes })
    }
}

/// Rewrite manifest scene entry points through the TOML CST, preserving comments.
pub fn rename_manifest_source(source: &str, block: &str, name: &str) -> Result<String, EditError> {
    if name == recite_core::ast::END_DIVERT_TARGET
        || name.contains("::")
        || !recite_core::is_valid_source_name(name)
    {
        return Err(EditError::Destination);
    }
    let loaded = recite_core::project::ProjectManifest::load_str("recite.project.toml", source);
    let manifest = loaded.manifest.ok_or(EditError::SourceRequired(
        "repair the project manifest before renaming",
    ))?;
    let mut document = source
        .parse::<toml_edit::DocumentMut>()
        .map_err(|_| EditError::SourceRequired("repair the project manifest before renaming"))?;
    if let Some(scenes) = document
        .get_mut("scenes")
        .and_then(toml_edit::Item::as_array_of_tables_mut)
    {
        for (index, scene) in manifest.scenes.iter().enumerate() {
            if scene.block == block {
                let field = scenes
                    .get_mut(index)
                    .and_then(|s| s.get_mut("block"))
                    .and_then(toml_edit::Item::as_value_mut)
                    .ok_or(EditError::Position)?;
                let decor = field.decor().clone();
                *field = toml_edit::Value::from(name);
                *field.decor_mut() = decor;
            }
        }
    }
    Ok(document.to_string())
}
