use super::super::{check_owner, parse};
use super::*;
use crate::declarations::{producer::Generated, recovery::Store};
use crate::project::read_regular;
use std::io::Write;

impl SourceWork {
    pub(super) fn prepare(
        &self,
        artifact: &mut Option<SourceArtifact>,
    ) -> Result<PreparedSource, FileError> {
        if self.token.cancelled() {
            return Err(FileError::DeclarationActionCancelled);
        }
        match &self.action {
            SourceAction::Inspect => Ok(PreparedSource::Unchanged),
            SourceAction::Bind(path) => self.bind(path),
            SourceAction::Reload => {
                let source = self.source.as_ref().ok_or(FileError::SchemaOwnership)?;
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
                Ok(PreparedSource::Replaced(Replacement {
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
            SourceAction::ExportDraft => {
                let source = self.source.as_ref().ok_or(FileError::SchemaOwnership)?;
                self.export_draft(source, artifact)?;
                Ok(PreparedSource::Unchanged)
            }
            SourceAction::ImportDraft(path) => {
                let source = self.source.as_ref().ok_or(FileError::SchemaOwnership)?;
                let draft = read_regular(&self.root.join(path))?;
                check_owner(
                    self.schema_output.last_valid_schema(),
                    parse(path, &draft)?.schema(),
                )?;
                Ok(PreparedSource::Replaced(Replacement {
                    baseline: source.baseline.clone(),
                    draft,
                    output: None,
                }))
            }
            SourceAction::Discard => {
                Ok(self
                    .source
                    .as_ref()
                    .map_or(PreparedSource::Unchanged, |source| {
                        PreparedSource::Replaced(Replacement {
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
        let editable = matches!(&self.action, SourceAction::ExportDraft);
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
    fn bind(&self, path: &Path) -> Result<PreparedSource, FileError> {
        let path = self.root.join(path).canonicalize()?;
        if path == self.output.canonicalize()? {
            return Err(FileError::SchemaOwnership);
        }
        if self
            .source
            .as_ref()
            .is_some_and(|source| source.path.canonicalize().is_ok_and(|old| old == path))
        {
            return Ok(PreparedSource::Unchanged);
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
        Ok(PreparedSource::Bound(Box::new(Source {
            path,
            baseline,
            draft,
            recovery,
            revision: DeclarationRevision::default(),
        })))
    }
}
