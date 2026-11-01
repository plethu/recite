use super::super::{check_owner, parse};
use super::*;
use crate::declarations::{producer::Generated, recovery::Store};
use crate::project::read_regular;
use std::io::Write;

impl DeclarationWork {
    pub(super) fn prepare(
        &self,
        artifact: &mut Option<SourceArtifact>,
    ) -> Result<PreparedDeclaration, FileError> {
        if self.token.cancelled() {
            return Err(FileError::DeclarationActionCancelled);
        }
        match &self.action {
            DeclarationAction::ReloadGenerated => self.reload_generated(),
            DeclarationAction::ReloadRegistration => Ok(self.reload_registration()),
            DeclarationAction::InspectSource => Ok(PreparedDeclaration::Unchanged),
            DeclarationAction::Bind(path) => self.bind(path),
            DeclarationAction::ReloadSource => {
                let source = self.inputs.source().ok_or(FileError::SchemaOwnership)?;
                let baseline = read_regular(&source.path)?;
                let output = read_regular(&self.output)?;
                let loaded = recite_core::schema::load_schema_manifest_str(
                    self.output.to_string_lossy(),
                    &output,
                );
                if let Some(schema) = &loaded.schema {
                    check_owner(self.schema_output.last_valid_schema(), schema)?;
                }
                if self.token.cancelled() {
                    return Err(FileError::DeclarationActionCancelled);
                }
                self.export_draft(source, artifact)?;
                Ok(PreparedDeclaration::Replaced(Replacement {
                    draft: baseline.clone(),
                    baseline,
                    output: Some(Arc::new(match loaded.schema {
                        Some(schema) => SchemaOutput::Ready(Generated {
                            text: output,
                            schema: Arc::new(schema),
                        }),
                        None => SchemaOutput::Damaged {
                            baseline: output,
                            last_valid: self.schema_output.last_valid_schema().clone(),
                        },
                    })),
                }))
            }
            DeclarationAction::ExportDraft => {
                let source = self.inputs.source().ok_or(FileError::SchemaOwnership)?;
                self.export_draft(source, artifact)?;
                Ok(PreparedDeclaration::Unchanged)
            }
            DeclarationAction::ImportDraft(path) => {
                let source = self.inputs.source().ok_or(FileError::SchemaOwnership)?;
                let draft = read_regular(&self.root.join(path))?;
                check_owner(
                    self.schema_output.last_valid_schema(),
                    parse(path, &draft)?.schema(),
                )?;
                Ok(PreparedDeclaration::Replaced(Replacement {
                    baseline: source.baseline.clone(),
                    draft,
                    output: None,
                }))
            }
            DeclarationAction::Discard => {
                Ok(self
                    .inputs
                    .source()
                    .map_or(PreparedDeclaration::Unchanged, |source| {
                        PreparedDeclaration::Replaced(Replacement {
                            baseline: source.baseline.clone(),
                            draft: source.baseline.clone(),
                            output: None,
                        })
                    }))
            }
        }
    }
    fn export_draft(
        &self,
        source: &SourceStamp,
        artifact: &mut Option<SourceArtifact>,
    ) -> Result<(), FileError> {
        let editable = matches!(&self.action, DeclarationAction::ExportDraft);
        let parent = source.path.parent().ok_or(FileError::Selection)?;
        let mut copy = tempfile::Builder::new()
            .prefix(if editable {
                ".recite-schema-repair-"
            } else {
                ".recite-schema-recovered-"
            })
            .suffix(".toml")
            .tempfile_in(parent)?;
        copy.write_all(source.draft.as_bytes())?;
        copy.as_file().sync_all()?;
        let (_, path) = copy.keep().map_err(|error| error.error)?;
        *artifact = Some(if editable {
            SourceArtifact::Editable(path)
        } else {
            SourceArtifact::Archived(path)
        });
        crate::sync_directory(parent).map_err(FileError::from)
    }
    fn bind(&self, path: &Path) -> Result<PreparedDeclaration, FileError> {
        let path = self.root.join(path).canonicalize()?;
        if path == self.output.canonicalize()? {
            return Err(FileError::SchemaOwnership);
        }
        if self
            .inputs
            .source()
            .is_some_and(|source| source.path.canonicalize().is_ok_and(|old| old == path))
        {
            return Ok(PreparedDeclaration::Unchanged);
        }
        let baseline = read_regular(&path)?;
        check_owner(
            self.schema_output.last_valid_schema(),
            parse(&path, &baseline)?.schema(),
        )?;
        let recovery = Store::open(&path)?;
        let (baseline, draft) = recovery
            .snapshot()
            .map(|snapshot| (snapshot.baseline.clone(), snapshot.draft.clone()))
            .unwrap_or_else(|| (baseline.clone(), baseline));
        check_owner(
            self.schema_output.last_valid_schema(),
            parse(&path, &baseline)?.schema(),
        )?;
        Ok(PreparedDeclaration::Bound(Box::new(Source {
            path,
            baseline,
            draft,
            recovery,
            revision: DeclarationRevision::default(),
        })))
    }
}
