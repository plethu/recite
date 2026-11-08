//! Renaming a line or choice's stable `@id` field.
//!
//! The compiler plans the header and catalogue edits. The workspace supplies
//! the project's current catalogues: it re-enumerates the manifest's
//! `[discovery].catalogs` and reads them only for this request, preferring an
//! open editor buffer so the edit is guarded by the version the client holds.

use lsp_types::{PrepareRenameResponse, Uri, WorkspaceEdit};
use recite_compiler::authoring::{
    AuthoringEditError, AuthoringKernel, CancellationToken, OpenDocument as KernelOpenDocument,
    SavedDocument as KernelSavedDocument, StableIdRefusal, StableIdTarget,
};
use recite_core::{DocumentKey, SourcePosition};
use recite_ui::MsgId;

use super::LspWorkspace;
use crate::edit_projection::{EditDocument, project_plan};
use crate::paths::file_path_to_uri;
use crate::position::source_range_to_lsp;

/// A rename the compiler or workspace refused, with a localised reason the
/// client shows to the author.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct RenameRefusal {
    pub(crate) message: String,
}

/// One catalogue loaded for a rename request.
struct LoadedCatalog {
    key: DocumentKey,
    uri: Uri,
    open_version: Option<i32>,
    text: String,
}

impl LspWorkspace {
    /// The `@id` range and current ID, when `position` is on a stable ID.
    pub(super) fn prepare_stable_id_rename(
        &self,
        partition: &str,
        key: &DocumentKey,
        position: SourcePosition,
        control: &CancellationToken,
    ) -> Option<PrepareRenameResponse> {
        let snapshot = self.partition(partition)?.kernel.snapshot();
        let occurrence = snapshot
            .query(control)
            .stable_id_occurrence(&StableIdTarget::position(key.clone(), position))
            .ok()?;
        let source = snapshot.document(key)?.source_index();
        Some(PrepareRenameResponse::RangeWithPlaceholder {
            range: source_range_to_lsp(source, occurrence.id_range())?,
            placeholder: occurrence.id().to_string(),
        })
    }

    /// Plans a stable-ID rename at `position`. `None` means the position is
    /// not on a stable ID, so the caller may try another rename.
    pub(super) fn stable_id_rename(
        &self,
        partition: &str,
        key: &DocumentKey,
        position: SourcePosition,
        new_name: &str,
        control: &CancellationToken,
    ) -> Option<Result<Option<WorkspaceEdit>, RenameRefusal>> {
        let partition_state = self.partition(partition)?;
        let target = StableIdTarget::position(key.clone(), position);
        partition_state
            .kernel
            .snapshot()
            .query(control)
            .stable_id_occurrence(&target)
            .ok()?;
        Some(self.plan_stable_id_rename(
            partition,
            &partition_state.kernel,
            &target,
            new_name,
            control,
        ))
    }

    fn plan_stable_id_rename(
        &self,
        partition: &str,
        kernel: &AuthoringKernel,
        target: &StableIdTarget,
        new_name: &str,
        control: &CancellationToken,
    ) -> Result<Option<WorkspaceEdit>, RenameRefusal> {
        let catalogs = self.load_catalogs(partition)?;
        let candidate = kernel
            .with_catalogs(
                catalogs
                    .iter()
                    .filter(|catalog| catalog.open_version.is_none())
                    .map(|catalog| KernelSavedDocument::new(catalog.key.clone(), &*catalog.text)),
                catalogs.iter().filter_map(|catalog| {
                    catalog.open_version.map(|version| {
                        KernelOpenDocument::new(
                            catalog.key.clone(),
                            recite_compiler::authoring::DocumentVersion::new(i64::from(version)),
                            &*catalog.text,
                        )
                    })
                }),
                control,
            )
            .map_err(|_| self.refusal(MsgId::LspRenameIdIncomplete, []))?;
        let query = candidate.snapshot().query(control);
        let plan = match query.plan_rename_stable_id(target, new_name) {
            Ok(plan) => plan,
            Err(AuthoringEditError::NoEdits) => return Ok(None),
            Err(error) => return Err(self.refuse(error)),
        };

        let mut documents = self.navigation_documents(partition);
        for catalog in &catalogs {
            let Some(source) = candidate.snapshot().catalog(&catalog.key) else {
                continue;
            };
            documents.push(EditDocument {
                key: source.key(),
                uri: &catalog.uri,
                text: source.source_text(),
                layer: source.layer(),
                version: source.version(),
            });
        }
        Ok(project_plan(&plan, &query, &documents))
    }

    fn load_catalogs(&self, partition: &str) -> Result<Vec<LoadedCatalog>, RenameRefusal> {
        let Some(manifest) = self.saved.manifest_for_partition(partition) else {
            return Ok(Vec::new());
        };
        let (discovered, diagnostics) = manifest.discover_catalogs();
        if !diagnostics.is_empty() {
            return Err(self.refusal(MsgId::LspRenameIdIncomplete, []));
        }
        let mut catalogs = Vec::with_capacity(discovered.len());
        for catalog in discovered {
            let unreadable = || {
                self.refusal(
                    MsgId::LspRenameIdCatalogUnreadable,
                    [("catalog", catalog.key().as_str().to_owned())],
                )
            };
            let uri = file_path_to_uri(catalog.path()).ok_or_else(unreadable)?;
            let (open_version, text) = match self.documents.document(&uri) {
                Some(open) => (Some(open.version()), open.text().to_owned()),
                None => (
                    None,
                    std::fs::read_to_string(catalog.path()).map_err(|_| unreadable())?,
                ),
            };
            catalogs.push(LoadedCatalog {
                key: catalog.key().clone(),
                uri,
                open_version,
                text,
            });
        }
        Ok(catalogs)
    }

    fn refuse(&self, error: AuthoringEditError) -> RenameRefusal {
        let catalog_name = |key: &DocumentKey| key.as_str().to_owned();
        match error {
            AuthoringEditError::InvalidStableId {
                id,
                reason: StableIdRefusal::ReservedCharacter(character),
            } => self.refusal(
                MsgId::LspRenameIdReserved,
                [("id", id), ("character", character.to_string())],
            ),
            AuthoringEditError::InvalidStableId { id, .. } => {
                self.refusal(MsgId::LspRenameIdMalformed, [("id", id)])
            }
            AuthoringEditError::StableIdCollision {
                id,
                document: Some(catalog),
            } => self.refusal(
                MsgId::LspRenameIdInCatalog,
                [("id", id.to_string()), ("catalog", catalog_name(&catalog))],
            ),
            AuthoringEditError::StableIdCollision { id, document: None } => {
                self.refusal(MsgId::LspRenameIdInUse, [("id", id.to_string())])
            }
            AuthoringEditError::MalformedCatalog { document } => self.refusal(
                MsgId::LspRenameIdCatalogMalformed,
                [("catalog", catalog_name(&document))],
            ),
            _ => self.refusal(MsgId::LspRenameIdIncomplete, []),
        }
    }

    fn refusal<const N: usize>(&self, id: MsgId, args: [(&str, String); N]) -> RenameRefusal {
        RenameRefusal {
            message: self.ui_catalog.format_pairs(id, args),
        }
    }
}
