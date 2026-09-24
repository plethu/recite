use super::*;
use crate::design::{PathField, PathKind, SubmitAction};
use freya::code_editor::*;

#[derive(Clone, Copy)]
pub(super) struct SourceActions {
    pub writer: Writer,
    pub editing: State<bool>,
}
impl PartialEq for SourceActions {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}
impl Component for SourceActions {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let mut editing = self.editing;
        let path = use_state(String::new);
        let input = use_a11y();
        let browse = use_a11y();
        let files = writer.files.read();
        let Some(session) = files.as_ref().and_then(|f| f.declarations.as_ref()) else {
            return rect();
        };
        if let Some(source) = &session.source {
            return rect()
                .spacing(t::SPACE_SM)
                .child(label().text(source.path.display().to_string()))
                .child(
                    label()
                        .text(text(if session.current() {
                            MsgId::WriterSchemaCurrent
                        } else {
                            MsgId::WriterSchemaStale
                        }))
                        .color(t::colors().muted),
                )
                .child(
                    Button::new()
                        .on_press(move |_| editing.set(true))
                        .child(text(MsgId::WriterEditDeclarationSource)),
                );
        }
        let bind = EventHandler::new(move |()| {
            let result = writer
                .files
                .write()
                .as_mut()
                .and_then(|f| f.declarations.as_mut())
                .map(|s| s.bind(std::path::Path::new(path.peek().as_str())));
            match result {
                Some(Ok(())) => editing.set(true),
                Some(Err(e)) => writer.message.error(e.to_string()),
                None => {}
            }
        });
        let submit = bind.clone();
        rect()
            .spacing(t::SPACE_MD)
            .width(Size::fill())
            .child(
                paragraph()
                    .width(Size::fill())
                    .span(Span::new(text(MsgId::WriterSchemaReadOnly))),
            )
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::Flex)
                    .spacing(t::SPACE_MD)
                    .child(rect().width(Size::flex(1.)).child(PathField {
                        value: path,
                        id: input,
                        browse_id: browse,
                        kind: PathKind::Schema,
                        enabled: true,
                        submit,
                    }))
                    .child(
                        Button::new()
                            .enabled(!path.read().trim().is_empty())
                            .on_press(move |_| bind.call(()))
                            .child(text(MsgId::WriterBindSource)),
                    ),
            )
    }
}
#[derive(Clone, Copy)]
pub(super) struct SourceEditor {
    pub writer: Writer,
    pub editing: State<bool>,
}
impl PartialEq for SourceEditor {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}
impl Component for SourceEditor {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let mut editing = self.editing;
        let mut editor = use_state(move || {
            let files = writer.files.peek();
            let draft = files
                .as_ref()
                .and_then(|f| f.declarations.as_ref())
                .and_then(|s| s.source.as_ref())
                .map_or("", |s| s.draft.as_str());
            data(draft, writer.dark)
        });
        let mut revision = use_state(|| 0);
        if let Some(source) = writer
            .files
            .read()
            .as_ref()
            .and_then(|p| p.declarations.as_ref())
            .and_then(|s| s.source.as_ref())
            && source.revision != *revision.peek()
        {
            editor.set(data(&source.draft, writer.dark));
            revision.set(source.revision);
        }
        let mut dark = use_state(move || writer.dark);
        if *dark.peek() != writer.dark {
            editor
                .write()
                .set_theme(crate::palette::syntax(writer.dark));
            editor.write().parse();
            dark.set(writer.dark);
        }
        let id = use_a11y();
        let source_viewport = crate::source_editor::EditorViewport::new();
        let submit_id = use_a11y();
        let mut narrow = use_state(|| false);
        use_side_effect(move || {
            let draft = editor.read().rope.to_string();
            let mut files = writer.files.write();
            if let Some(source) = files
                .as_mut()
                .and_then(|f| f.declarations.as_mut())
                .and_then(|s| s.source.as_mut())
            {
                source.draft = draft;
                if let Err(error) = source.queue_recovery() {
                    writer.message.error(error.to_string());
                }
            }
        });
        let submit = SubmitAction {
            id: submit_id,
            caption: text(MsgId::WriterGenerateSchema),
            enabled: true,
            action: EventHandler::new(move |()| {
                let result = (|| {
                    let mut files = writer.files.write();
                    let project = files.as_mut().ok_or("Open a project first.".to_owned())?;
                    let session = project
                        .declarations
                        .as_mut()
                        .ok_or("Open the declaration source first.".to_owned())?;
                    if let Some(source) = &mut session.source {
                        source.draft = editor.peek().rope.to_string();
                    }
                    session.save_and_generate().map_err(|e| e.to_string())?;
                    if let Ok(model) = writer.buffers.model.write().as_mut() {
                        project.refresh(model).map_err(|e| e.to_string())?;
                    }
                    Ok(())
                })();
                writer
                    .message
                    .report(result, text(MsgId::WriterSourceSaved));
            }),
        };
        let keyboard = submit.clone();
        rect()
            .width(Size::fill())
            .height(Size::flex(1.))
            .on_sized(move |event: Event<SizedEventData>| {
                narrow.set_if_modified(event.area.width() < 900.);
            })
            .content(Content::Flex)
            .spacing(t::SPACE_MD)
            .on_key_down(move |e: Event<KeyboardEventData>| {
                if crate::design::keyboard::submit_key(&e) || crate::editing::is_save_key(&e) {
                    e.prevent_default();
                    e.stop_propagation();
                    keyboard.run();
                }
            })
            .child(
                rect().width(Size::fill()).height(Size::flex(1.)).child(
                    crate::source_editor::EditorSurface {
                        editor,
                        viewport: source_viewport,
                        id,
                        size: t::code_size(),
                        content: CodeEditor::new(editor, id)
                            .scroll_controller(source_viewport.scroll)
                            .font_family("monospace")
                            .font_size(t::code_size())
                            .gutter(false)
                            .on_pre_key_down(move |e: Event<KeyboardEventData>| {
                                if crate::design::keyboard::submit_key(&e)
                                    || crate::editing::is_save_key(&e)
                                {
                                    return false;
                                }
                                if e.key == Key::Named(NamedKey::Tab) {
                                    return false;
                                }
                                if e.key == Key::Named(NamedKey::Escape) {
                                    id.request_unfocus();
                                    return false;
                                }
                                e.stop_propagation();
                                true
                            })
                            .into_element(),
                    },
                ),
            )
            .child(
                rect()
                    .direction(if *narrow.read() {
                        Direction::Vertical
                    } else {
                        Direction::Horizontal
                    })
                    .spacing(t::SPACE_MD)
                    .child(submit.button())
                    .child(
                        Button::new()
                            .flat()
                            .on_press(move |_| {
                                let mut files = writer.files.write();
                                if let Some(session) =
                                    files.as_mut().and_then(|f| f.declarations.as_mut())
                                {
                                    if let Some(source) = &mut session.source {
                                        source.draft = editor.peek().rope.to_string();
                                    }
                                    match session.reload_source() {
                                        Ok(copy) => {
                                            if let Some(source) = &session.source {
                                                editor.set(data(&source.draft, writer.dark));
                                            }
                                            writer.message.info(format!(
                                                "Source reloaded. Previous draft retained at {}",
                                                copy.display()
                                            ));
                                        }
                                        Err(e) => writer.message.error(e.to_string()),
                                    }
                                }
                            })
                            .child(text(MsgId::WriterReloadSchemaSource)),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .on_press(move |_| {
                                let mut files = writer.files.write();
                                if let Some(session) =
                                    files.as_mut().and_then(|f| f.declarations.as_mut())
                                {
                                    if let Err(error) = session.discard() {
                                        writer.message.error(error.to_string());
                                    }
                                    if let Some(source) = &session.source {
                                        editor.set(data(&source.draft, writer.dark));
                                    }
                                }
                            })
                            .child(text(MsgId::WriterDiscard)),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .on_press(move |_| editing.set(false))
                            .child(text(MsgId::WriterBackDeclarations)),
                    ),
            )
    }
}
fn data(text: &str, dark: bool) -> CodeEditorData {
    crate::editing::editor_data(text, false, dark)
}
