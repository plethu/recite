//! Editor positions are session-local and restored only against identical text.
use freya::{
    code_editor::{CodeEditorData, Rope},
    prelude::*,
    text_edit::{TextEditor, TextSelection},
};
use std::collections::BTreeMap;

struct Position {
    text: Rope,
    selection: TextSelection,
}
#[derive(Clone, Copy)]
pub(crate) struct Bookmarks(State<BTreeMap<String, Position>>);
impl Bookmarks {
    pub fn new() -> Self {
        Self(use_state(BTreeMap::new))
    }
    pub fn remember(mut self, key: &str, editor: &CodeEditorData) {
        self.0.write().insert(
            key.into(),
            Position {
                text: editor.rope.clone(),
                selection: editor.selection().clone(),
            },
        );
    }
    pub fn restore(self, key: &str, editor: &mut CodeEditorData) {
        if let Some(position) = self.0.peek().get(key)
            && position.text == editor.rope
        {
            *editor.selection_mut() = position.selection.clone();
        }
    }
}
