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

pub(crate) fn perform(
    buffers: crate::buffers::Buffers,
    mut message: crate::feedback::Feedback,
    dark: bool,
    action: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>,
) -> Result<(), String> {
    buffers.remember();
    let mut model = buffers.model;
    let mut editor = buffers.editor;
    let mut prose = buffers.prose;
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
                if session.view() == &View::Source {
                    buffers
                        .bookmarks
                        .restore(session.document().key().as_str(), &mut editor.write());
                }
                prose.set_if_modified(session.draft().to_owned());
            })
        }
        Err(error) => {
            message.error(error.to_string());
            return Err(error.to_string());
        }
    };
    let outcome = outcome.map_err(|error| error.to_string());
    message.report(outcome.clone(), String::new());
    outcome
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Pane {
    Map,
    Script,
    Preview,
    Rules,
    Declarations,
    Disk,
    Rename,
    Build,
}

#[derive(Clone, Copy)]
pub(crate) struct Writer {
    pub command_search: crate::commands::Search,
    pub source_viewport: crate::source_editor::EditorViewport,
    pub layout: crate::presentation::Layout,
    pub trial: crate::preview_panel::TrialInputs,
    pub localisation: State<crate::localisation::Localisation>,
    pub files: State<Option<crate::project::ProjectFiles>>,
    pub examples: State<std::collections::BTreeMap<String, Workbench>>,
    pub queue: crate::navigation::Queue,
    pub reference: State<Option<crate::reading_context::Reference>>,
    pub buffers: crate::buffers::Buffers,
    pub message: crate::feedback::Feedback,
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

    pub fn scene_opened(mut self) -> Result<(), String> {
        let recovered_source = self
            .buffers
            .model
            .peek()
            .as_ref()
            .is_ok_and(|m| m.view() == &View::Source && m.has_draft());
        if recovered_source {
            self.layout.view.set(recite_config::WriterView::Source);
            self.pane.set(Pane::Map);
            return Ok(());
        }
        self.selection.set(None);
        self.layout
            .view
            .set(self.preferences.peek().config.writer.view);
        if *self.layout.view.peek() == recite_config::WriterView::Source {
            self.try_navigate(|m| m.select(View::Source))
        } else {
            self.try_navigate(Workbench::show_script)?;
            if self.layout.standalone() {
                self.ensure_beat()?;
            }
            Ok(())
        }
    }
    pub fn ensure_beat(mut self) -> Result<(), String> {
        if self
            .buffers
            .model
            .peek()
            .as_ref()
            .is_ok_and(|m| m.view() != &View::Source && m.selected_block().ok().flatten().is_some())
        {
            self.pane.set(Pane::Script);
            return Ok(());
        }
        let id = {
            let state = self.buffers.model.peek();
            let model = state.as_ref().map_err(|e| e.to_string())?;
            let blocks = model
                .document()
                .script_snapshot()
                .map_err(|e| e.to_string())?;
            model
                .selected_block()
                .ok()
                .flatten()
                .or_else(|| self.selection.peek().clone())
                .filter(|id| blocks.iter().any(|b| &b.id == id))
                .or_else(|| {
                    blocks
                        .iter()
                        .find(|b| b.is_default)
                        .or(blocks.first())
                        .map(|b| b.id.clone())
                })
        };
        if let Some(id) = id {
            self.try_navigate(|m| m.inspect_block(&id))?;
            self.selection.set(Some(id));
        }
        self.pane.set(Pane::Script);
        Ok(())
    }
    pub fn history_step(self, forward: bool) {
        crate::navigation::step(self, forward);
    }
    pub fn pin(mut self) {
        if self.try_navigate(|_| Ok(())).is_err() {
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
        if self.try_navigate(|_| Ok(())).is_ok() {
            self.layout.view.set(recite_config::WriterView::Map);
            self.pane.set(Pane::Map);
            self.map_focus.request_focus();
        }
    }
    pub fn set_view(mut self, view: recite_config::WriterView) {
        let outcome = self.try_navigate(|m| match view {
            recite_config::WriterView::Script | recite_config::WriterView::Map => m.show_script(),
            recite_config::WriterView::Source => m.select(View::Source),
        });
        if outcome.is_ok() {
            self.localisation.write().active = false;
            self.layout.view.set(view);
            self.pane.set(Pane::Map);
            if view == recite_config::WriterView::Script
                && let Err(error) = self.ensure_beat()
            {
                self.message.error(error);
            }
            if let Err(e) = self
                .preferences
                .write()
                .update(recite_config::UserConfigEdit::WriterView(view))
            {
                self.message.error(e);
            }
        }
    }
    /// Commit a prose editing session before changing context; source drafts stay explicit.
    pub fn navigate(self, action: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>) {
        let _ = self.try_navigate(action);
    }

    pub fn try_navigate(
        self,
        action: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>,
    ) -> Result<(), String> {
        self.try_perform(|session| {
            if matches!(session.view(), View::Passage(_)) && session.has_draft() {
                session.apply()?;
            }
            action(session)
        })
    }

    pub fn perform(self, action: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>) {
        let _ = self.try_perform(action);
    }

    pub fn try_perform(
        self,
        action: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>,
    ) -> Result<(), String> {
        perform(self.buffers, self.message, self.dark, action)
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
    crate::commands::shortcut(event).is_some()
        || event.code == Code::F6
        || (event.code == Code::Comma
            && event.modifiers
                == if cfg!(target_os = "macos") {
                    Modifiers::META
                } else {
                    Modifiers::CONTROL
                })
}
