use freya::{code_editor::*, prelude::*};
use recite_writer_model::{View, Workbench, WorkbenchError};

pub type Session = State<Result<Workbench, WorkbenchError>>;

pub fn editor_data(text: &str, source: bool, dark: bool) -> CodeEditorData {
    let language = source.then(|| {
        EditorLanguage::new(
            recite_writer_grammar::LANGUAGE,
            recite_writer_grammar::HIGHLIGHTS,
        )
    });
    let mut data = CodeEditorData::new(Rope::from_str(text), language);
    data.set_theme(super::palette::syntax(dark));
    data.parse();
    data.measure(
        if source { 15. } else { 20. },
        if source { "monospace" } else { "serif" },
    );
    data
}

pub fn perform(
    mut model: Session,
    mut editor: State<CodeEditorData>,
    mut message: State<String>,
    mut prose: State<String>,
    dark: bool,
    action: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>,
) {
    let mut state = model.write();
    let outcome = match state.as_mut() {
        Ok(session) => {
            session.set_draft(if matches!(session.view(), View::Source) {
                editor.peek().rope.to_string()
            } else {
                prose.peek().clone()
            });
            let previous_view = session.view().clone();
            action(session).map(|()| {
                if session.view() != &previous_view || session.draft() != editor.peek().rope {
                    editor.set(editor_data(
                        session.draft(),
                        matches!(session.view(), View::Source),
                        dark,
                    ));
                }
                prose.set_if_modified(session.draft().to_owned());
                String::new()
            })
        }
        Err(error) => {
            message.set(error.to_string());
            return;
        }
    };
    message.set(match outcome {
        Ok(text) => text,
        Err(error) => error.to_string(),
    });
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Pane {
    Map,
    Script,
    Preview,
}

#[derive(Clone, Copy)]
pub(crate) struct Writer {
    pub localisation: State<crate::localisation::Localisation>,
    pub files: State<Option<crate::project::ProjectFiles>>,
    pub examples: State<std::collections::BTreeMap<String, Workbench>>,
    pub queue: crate::navigation::Queue,
    pub reference: State<Option<crate::reading_context::Reference>>,
    pub buffers: crate::buffers::Buffers,
    pub message: State<String>,
    pub dark: bool,
    pub scroll: ScrollController,
    pub pane: State<Pane>,
    pub preferences: State<crate::preferences::Preferences>,
    pub settings_open: State<bool>,
    pub map_focus: AccessibilityId,
    pub sidebar_focus: AccessibilityId,
    pub inspector_focus: AccessibilityId,
    pub search_focus: AccessibilityId,
    pub selection: State<Option<String>>,
    pub search: State<String>,
    pub expanded: State<std::collections::BTreeSet<(String, String, String)>>,
}

impl Writer {
    pub fn inspect(mut self, block: &str) {
        self.navigate(|m| m.inspect_block(block));
        if self
            .buffers
            .model
            .peek()
            .as_ref()
            .is_ok_and(|m| m.selected_block().ok().flatten().as_deref() == Some(block))
        {
            self.selection.set(Some(block.to_owned()));
            self.pane.set(Pane::Script);
            self.inspector_focus.request_focus();
            self.scroll
                .scroll_to(ScrollPosition::Start, Direction::Vertical);
        }
    }

    pub fn scene_opened(mut self) {
        self.selection.set(None);
        match self.preferences.peek().config.writer.view {
            recite_config::WriterView::Map => self.navigate(Workbench::show_script),
            recite_config::WriterView::Source => self.navigate(|m| m.select(View::Source)),
        }
    }

    pub fn history_step(self, forward: bool) {
        crate::navigation::step(self, forward);
    }
    pub fn pin(mut self) {
        self.navigate(|_| Ok(()));
        if !self.message.peek().is_empty() {
            return;
        }
        if let Ok(session) = self.buffers.model.peek().as_ref()
            && let Ok(Some(id)) = session.selected_block()
            && let Ok(blocks) = session.document().script_snapshot()
            && let Some(block) = blocks.iter().find(|b| b.id == id)
        {
            self.reference.set(Some(crate::reading_context::Reference {
                document: session.document().key().to_string(),
                block: block.clone(),
            }));
        }
    }
    pub fn close_editor(mut self) {
        self.navigate(|_| Ok(()));
        if self.message.peek().is_empty() {
            self.pane.set(Pane::Map);
            self.map_focus.request_focus();
        }
    }
    pub fn set_view(mut self, view: recite_config::WriterView) {
        self.navigate(|m| match view {
            recite_config::WriterView::Map => m.show_script(),
            recite_config::WriterView::Source => m.select(View::Source),
        });
        if self.message.peek().is_empty() {
            self.pane.set(Pane::Map);
            if let Err(e) = self
                .preferences
                .write()
                .update(recite_config::UserConfigEdit::WriterView(view))
            {
                self.message.set(e);
            }
        }
    }
    /// Commit a prose editing session before changing context; source drafts stay explicit.
    pub fn navigate(self, action: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>) {
        self.perform(|session| {
            if matches!(session.view(), View::Passage(_)) && session.has_draft() {
                session.apply()?;
            }
            action(session)
        });
    }

    pub fn perform(self, action: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>) {
        perform(
            self.buffers.model,
            self.buffers.editor,
            self.message,
            self.buffers.prose,
            self.dark,
            action,
        );
    }
}

/// One save shortcut policy shared by text fields and the project toolbar.
pub(super) fn is_save_key(event: &KeyboardEventData) -> bool {
    let modifier = if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    };
    event.code == Code::KeyS && event.modifiers == modifier
}

pub(super) fn is_workspace_key(event: &KeyboardEventData) -> bool {
    event.code == Code::F6
        || (event.code == Code::Comma
            && event.modifiers
                == if cfg!(target_os = "macos") {
                    Modifiers::META
                } else {
                    Modifiers::CONTROL
                })
}
