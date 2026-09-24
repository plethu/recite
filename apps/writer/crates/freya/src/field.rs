//! Explicit source editing; prose fields manage their own focus and edit sessions.
use crate::design::tokens as t;
use crate::editing::Writer;
use freya::{code_editor::*, prelude::*};

pub(super) fn render(writer: Writer, editor_id: AccessibilityId) -> Element {
    SourceField { writer, editor_id }.into_element()
}
#[derive(Clone)]
struct SourceField {
    writer: Writer,
    editor_id: AccessibilityId,
}
impl PartialEq for SourceField {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark && self.editor_id == other.editor_id
    }
}
impl Component for SourceField {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let editor_id = self.editor_id;
        let mut viewport = use_state(|| None::<Area>);
        let mut field = rect()
            .a11y_role(AccessibilityRole::Group)
            .a11y_alt("Source editor")
            .width(Size::fill())
            .height(Size::fill())
            .content(Content::Flex)
            .spacing(t::SPACE_SM)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .on_pointer_down(move |_| editor_id.request_focus())
                    .on_sized(move |event: Event<SizedEventData>| {
                        viewport.set_if_modified(Some(event.area))
                    })
                    .child(crate::source_editor::EditorSurface {
                        editor: writer.buffers.editor,
                        viewport: writer.source_viewport,
                        id: editor_id,
                        size: t::code_size(),
                        content: CodeEditor::new(writer.buffers.editor, editor_id)
                            .scroll_controller(writer.source_viewport.scroll)
                            .font_family("monospace")
                            .font_size(t::code_size())
                            .line_height(t::CODE_LINE_HEIGHT)
                            .gutter(false)
                            .show_whitespace(false)
                            .on_pre_key_down(move |event: Event<KeyboardEventData>| {
                                if matches!(
                                    event.key,
                                    Key::Named(
                                        NamedKey::Control
                                            | NamedKey::Meta
                                            | NamedKey::Alt
                                            | NamedKey::Shift
                                    )
                                ) {
                                    return false;
                                }
                                if crate::field_completion::keyboard(writer, &event) {
                                    return false;
                                }
                                if crate::design::keyboard::submit_key(&event) {
                                    return false;
                                }
                                if event.code == Code::Space
                                    && event.modifiers == Modifiers::CONTROL
                                {
                                    return false;
                                }
                                if crate::editing::is_workspace_key(&event) {
                                    return false;
                                }
                                if crate::editing::is_save_key(&event) {
                                    return false;
                                }
                                if crate::closing::is_quit_key(&event) {
                                    crate::closing::keyboard(event);
                                    return false;
                                }
                                match &event.key {
                                    Key::Named(NamedKey::Tab) => false,
                                    Key::Named(NamedKey::Escape) => {
                                        editor_id.request_unfocus();
                                        event.stop_propagation();
                                        false
                                    }
                                    _ => {
                                        event.stop_propagation();
                                        true
                                    }
                                }
                            })
                            .into_element(),
                    }),
            );
        let diagnostics = writer
            .buffers
            .model
            .read()
            .as_ref()
            .map(|model| model.document().diagnostics())
            .unwrap_or_default();
        if !diagnostics.is_empty() {
            field = field.child(
                ScrollView::new()
                    .width(Size::fill())
                    .height(Size::px((diagnostics.len().min(3) as f32 * 48.).max(48.)))
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding(t::SPACE_SM)
                            .background(crate::design::palette::current().inset)
                            .border(
                                Border::new()
                                    .width(1.)
                                    .fill(crate::design::palette::current().rule),
                            )
                            .children(
                                diagnostics
                                    .iter()
                                    .map(|diagnostic| {
                                        crate::diagnostics::row(writer, editor_id, diagnostic)
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                    ),
            );
        }
        field
            .child(crate::field_actions::SourceActions {
                writer,
                completion: crate::field_completion::Completion {
                    writer,
                    editor_id,
                    viewport,
                }
                .into_element(),
            })
            .into_element()
    }
}
