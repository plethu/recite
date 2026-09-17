//! Explicit source editing; prose fields manage their own focus and edit sessions.
use crate::design::Button;
use crate::design::tokens as t;
use crate::editing::Writer;
use freya::{code_editor::*, prelude::*};
use recite_writer_model::{View, Workbench};

pub(super) fn render(writer: Writer, editor_id: AccessibilityId) -> Element {
    let mut field = rect()
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt("Source editor")
        .width(Size::fill())
        .height(Size::fill())
        .content(Content::Flex)
        .spacing(t::SPACE_SM)
        .child(
            rect().width(Size::fill()).height(Size::flex(1.)).child(
                CodeEditor::new(writer.buffers.editor, editor_id)
                    .font_family("monospace")
                    .font_size(t::TEXT_CODE)
                    .gutter(true)
                    .show_whitespace(false)
                    .on_pre_key_down(move |event: Event<KeyboardEventData>| {
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
                    }),
            ),
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
                .height(Size::px(140.))
                .child(
                    rect().width(Size::fill()).children(
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
    if writer
        .buffers
        .model
        .peek()
        .as_ref()
        .is_ok_and(|m| m.view() == &View::Source && m.has_draft())
    {
        field = field.child(
            rect()
                .horizontal()
                .spacing(t::SPACE_SM)
                .child(
                    Button::new()
                        .on_press(move |_| writer.perform(Workbench::apply))
                        .child("Apply draft"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| {
                            writer.perform(|m| {
                                m.discard();
                                Ok(())
                            })
                        })
                        .child("Discard draft"),
                ),
        );
    }
    field.into_element()
}
