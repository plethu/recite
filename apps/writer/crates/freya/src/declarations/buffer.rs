//! Actions harvest the mounted widget explicitly; observers may still be queued.
use crate::editing::Writer;
use freya::{code_editor::CodeEditorData, prelude::*};
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) struct SourceBuffer {
    pub(super) path: PathBuf,
    pub(super) editor: State<CodeEditorData>,
    pub(super) revision: State<u64>,
}

pub(crate) fn harvest(mut writer: Writer) {
    let Some(slot) = writer.declaration_buffer.try_peek() else {
        return;
    };
    let Some(buffer) = slot.as_ref() else {
        return;
    };
    let (Some(editor), Some(revision), Some(mut files)) = (
        buffer.editor.try_peek(),
        buffer.revision.try_peek(),
        writer.files.try_write(),
    ) else {
        return;
    };
    let Some(source) = files
        .as_mut()
        .and_then(|p| p.declarations.as_mut())
        .and_then(|s| s.source.as_mut())
        .filter(|s| s.path == buffer.path && s.revision == *revision)
    else {
        return;
    };
    let draft = editor.rope.to_string();
    if source.draft != draft {
        source.draft = draft;
        // Recovery failure must not veto a rescue Save of the captured draft.
        if let Err(error) = source.queue_recovery() {
            writer.message.error(error.to_string());
        }
    }
}
