//! One source file, its authoring model, and its existing durable recovery lease.
use crate::recovery::{Recovery, RecoveryStore};
use crate::{FileError, export_recovery, read_regular, replace_checked};
use recite_core::DocumentKey;
use recite_writer_model::{
    ConditionValue, Document, EffectAck, PreparedPreview, Preview, ProjectContext, View, Workbench,
    WorkbenchError,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct DocumentSession {
    path: PathBuf,
    baseline: Arc<str>,
    model: Workbench,
    recovery: RecoveryStore,
}

impl DocumentSession {
    pub fn open(
        path: PathBuf,
        key: DocumentKey,
        context: ProjectContext,
    ) -> Result<Self, FileError> {
        Self::open_using(path, |source| Document::in_project(key, source, context))
    }

    pub(crate) fn open_sibling(
        path: PathBuf,
        key: DocumentKey,
        context: ProjectContext,
        sibling: &Document,
    ) -> Result<Self, FileError> {
        Self::open_using(path, |source| {
            sibling.open_sibling_with_control(
                key,
                source,
                context,
                &recite_compiler::authoring::CancellationToken::new(),
            )
        })
    }

    fn open_using(
        path: PathBuf,
        document: impl FnOnce(&str) -> Result<Document, recite_writer_model::EditError>,
    ) -> Result<Self, FileError> {
        let saved: Arc<str> = read_regular(&path)?.into();
        let recovery = RecoveryStore::open(&path)?;
        let recovered = recovery.snapshot();
        let source = recovered.map_or(saved.as_ref(), |value| value.draft.source());
        let document = document(source).map_err(WorkbenchError::from)?;
        let initial = recovered.map(|value| value.draft.view().clone());
        let mut model = Workbench::from_document(document, initial)?;
        if let Some(value) = recovered {
            value.draft.restore(&mut model)?;
        }
        let baseline = recovered.map_or(saved, |value| value.baseline.clone());
        Ok(Self {
            path,
            baseline,
            model,
            recovery,
        })
    }

    pub(crate) fn refresh(&mut self, context: ProjectContext) -> Result<(), FileError> {
        self.model.refresh_project(context)?;
        Ok(())
    }

    pub(crate) fn park(&mut self) -> Result<(), FileError> {
        self.checkpoint()?;
        self.recovery.park()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn model(&self) -> &Workbench {
        &self.model
    }

    /// Trial state is transient; playback never queues a document recovery write.
    pub fn accept_preview(
        &mut self,
        preview: Box<PreparedPreview>,
    ) -> Result<Option<Preview>, Box<PreparedPreview>> {
        self.model.accept_preview(preview)
    }

    pub fn advance_preview(&mut self, choice: Option<usize>) -> Result<(), WorkbenchError> {
        self.model.advance_preview(choice)
    }

    pub fn answer_preview(&mut self, value: ConditionValue) -> Result<(), WorkbenchError> {
        self.model.answer_preview(value)
    }

    pub fn acknowledge_preview(&mut self, ack: EffectAck) -> Result<(), WorkbenchError> {
        self.model.acknowledge_preview(ack)
    }
    pub fn dirty(&self) -> bool {
        self.model.has_draft() || self.model.document().source() != self.baseline.as_ref()
    }
    pub fn recovery_error(&self) -> Option<String> {
        self.recovery.error()
    }

    fn snapshot(&self) -> Option<Recovery> {
        self.dirty()
            .then(|| Recovery::new(self.baseline.clone(), self.model.recovery()))
    }

    /// Every model mutation queues recovery, including an operation that partially fails.
    pub fn edit(
        &mut self,
        action: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>,
    ) -> Result<(), FileError> {
        let result = action(&mut self.model).map_err(FileError::from);
        let recovery = self.recovery.queue(self.snapshot());
        result.and(recovery)
    }

    pub fn checkpoint(&mut self) -> Result<(), FileError> {
        self.recovery.persist(self.snapshot())
    }

    pub fn save(&mut self) -> Result<(), FileError> {
        self.edit(Workbench::apply)?;
        self.checkpoint()?;
        let source = self.model.document().source_snapshot();
        replace_checked(&self.path, &self.baseline, &source)?;
        self.baseline = source;
        self.recovery.persist(None)
    }

    /// Save accepted source for an IDE without requiring the current field draft to apply.
    pub fn prepare_external_edit(&mut self) -> Result<(), FileError> {
        self.checkpoint()?;
        let source = self.model.document().source_snapshot();
        replace_checked(&self.path, &self.baseline, &source)?;
        self.baseline = source;
        self.checkpoint()
    }

    /// Explicitly accept disk bytes; preserve local work in an independent JSON copy first.
    pub fn reload(&mut self, context: ProjectContext) -> Result<Option<PathBuf>, FileError> {
        let key = self.model.document().key().clone();
        if !context
            .documents
            .iter()
            .any(|document| document.key() == &key)
        {
            return Err(FileError::SessionExcluded(self.path.clone()));
        }
        let disk: Arc<str> = read_regular(&self.path)?.into();
        let document =
            Document::in_project(key, disk.as_ref(), context).map_err(WorkbenchError::from)?;
        let mut next = Workbench::from_document(document, None)?;
        if let View::Passage(id) = self.model.view()
            && next
                .document()
                .passage_snapshot()
                .is_ok_and(|passages| passages.iter().any(|passage| &passage.id == id))
        {
            next.select(View::Passage(id.clone()))?;
        }
        self.checkpoint()?;
        let copy = self
            .snapshot()
            .map(|value| export_recovery(&self.path, &value))
            .transpose()?;
        self.recovery.persist(None)?;
        self.baseline = disk;
        self.model = next;
        Ok(copy)
    }
}
