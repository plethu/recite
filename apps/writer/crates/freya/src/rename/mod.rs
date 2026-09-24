use crate::{
    design::{
        Button, ComparisonRow, ComparisonView, PickerOption, SearchPicker, SubmitAction,
        tokens as t,
    },
    editing::{Pane, Writer},
    messages::{MsgId, text},
};
use freya::prelude::*;
pub(crate) fn open(mut writer: Writer) {
    writer.buffers.harvest();
    writer.pane.set(Pane::Rename);
}
#[derive(Clone, Copy)]
pub(crate) struct RenameScreen {
    pub writer: Writer,
}
impl PartialEq for RenameScreen {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}
impl Component for RenameScreen {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let mut beat = use_state(move || {
            writer
                .buffers
                .model
                .peek()
                .as_ref()
                .ok()
                .and_then(|m| m.selected_block().ok().flatten())
                .unwrap_or_default()
        });
        let name = use_state(String::new);
        let query = use_state(String::new);
        let picker_open = use_state(|| false);
        let field_id = use_a11y();
        let picker_id = use_a11y();
        let input_id = use_a11y();
        let submit_id = use_a11y();
        let mut body = rect()
            .width(Size::fill())
            .spacing(t::SPACE_LG)
            .padding(t::SPACE_XL)
            .child(
                rect()
                    .horizontal()
                    .spacing(t::SPACE_LG)
                    .child(
                        label()
                            .text(text(MsgId::WriterRenameProject))
                            .font_size(t::title()),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .on_press(move |_| writer.pane.set(Pane::Map))
                            .child(text(MsgId::WriterReturnWriting)),
                    ),
            );
        let files = writer.files.read();
        let Some(project) = files.as_ref() else {
            return body.child(label().text(crate::messages::text(
                crate::messages::MsgId::WriterGuiOpenAProjectFirst,
            )));
        };
        let primary = if let Some(review) = &project.rename_review {
            body = body
                .child(
                    paragraph()
                        .width(Size::fill())
                        .span(Span::new(text(MsgId::WriterRenameHint))),
                )
                .child(ComparisonView {
                    code: true,
                    before: text(MsgId::WriterPreviousSource),
                    after: text(MsgId::WriterProposedSource),
                    rows: review
                        .plan
                        .changes
                        .iter()
                        .flat_map(|change| {
                            ComparisonRow::source(
                                change.document.as_str(),
                                &change.before,
                                &change.after,
                            )
                        })
                        .chain(
                            (review.manifest_before != review.manifest_after)
                                .then(|| {
                                    ComparisonRow::source(
                                        "recite.project.toml",
                                        &review.manifest_before,
                                        &review.manifest_after,
                                    )
                                })
                                .into_iter()
                                .flatten(),
                        )
                        .collect(),
                })
                .child(
                    Button::new()
                        .flat()
                        .on_press(move |_| {
                            if let Some(project) = writer.files.write().as_mut() {
                                project.rename_review = None;
                            }
                        })
                        .child(text(MsgId::WriterCancel)),
                );
            SubmitAction {
                id: submit_id,
                caption: text(MsgId::WriterRenameApply),
                enabled: !review.plan.changes.is_empty(),
                action: EventHandler::new(move |()| {
                    writer.buffers.harvest();
                    let result = {
                        let mut model = writer.buffers.model.write();
                        let mut files = writer.files.write();
                        match (model.as_mut(), files.as_mut()) {
                            (Ok(model), Some(project)) => {
                                project.apply_rename(model).map_err(|e| e.to_string())
                            }
                            _ => Err("Open a project first.".into()),
                        }
                    };
                    writer.buffers.sync(writer.dark);
                    if result.is_ok() {
                        writer.pane.set(Pane::Map);
                    }
                    writer
                        .message
                        .report(result, text(MsgId::WriterRenameApplied));
                }),
            }
        } else {
            let options = writer
                .buffers
                .model
                .read()
                .as_ref()
                .ok()
                .and_then(|m| m.document().script().ok())
                .unwrap_or_default()
                .into_iter()
                .filter(|b| {
                    b.id.to_lowercase()
                        .contains(&query.read().trim().to_lowercase())
                })
                .map(|b| PickerOption {
                    annotation: String::new(),
                    title: b.id.clone(),
                    value: b.id,
                    detail: String::new(),
                })
                .collect();
            body = body
                .child(SearchPicker {
                    id: picker_id,
                    input_id,
                    name: text(MsgId::WriterRenameFrom),
                    placeholder: text(MsgId::WriterRenameFrom),
                    empty_hint: text(MsgId::WriterRenameFrom),
                    no_matches: text(MsgId::WriterNoMatches),
                    selected: beat.read().clone(),
                    query,
                    open: picker_open,
                    options: std::sync::Arc::new(options),
                    choose: EventHandler::new(move |v: PickerOption| beat.set(v.value)),
                    vim: writer.preferences.read().config.ui.keymap == recite_config::Keymap::Vim,
                    enabled: true,
                })
                .child(label().text(text(MsgId::WriterRenameName)))
                .child(
                    Input::new(name)
                        .placeholder(text(MsgId::WriterRenameName))
                        .a11y_id(field_id)
                        .width(Size::fill())
                        .on_pre_key_down(crate::closing::text_input_key),
                );
            SubmitAction {
                id: submit_id,
                caption: text(MsgId::WriterRenameReview),
                enabled: !beat.read().is_empty()
                    && !name.read().trim().is_empty()
                    && beat.read().as_str() != name.read().as_str(),
                action: EventHandler::new(move |()| {
                    writer.buffers.harvest();
                    let result = {
                        let mut files = writer.files.write();
                        let mut model = writer.buffers.model.write();
                        match (files.as_mut(), model.as_mut()) {
                            (Some(project), Ok(model)) => project
                                .review_rename(model, &beat.peek(), &name.peek())
                                .map_err(|e| e.to_string()),
                            _ => Err("Open a project first.".into()),
                        }
                    };
                    writer.message.report(result, String::new());
                }),
            }
        };
        let shortcut = primary.clone();
        body = body
            .child(primary.button())
            .on_key_down(move |e: Event<KeyboardEventData>| {
                if crate::design::keyboard::submit_key(&e) {
                    e.prevent_default();
                    e.stop_propagation();
                    shortcut.run();
                }
            });
        rect().width(Size::fill()).height(Size::fill()).child(
            ScrollView::new()
                .width(Size::fill())
                .height(Size::fill())
                .child(body),
        )
    }
}

pub(crate) fn history(mut writer: Writer, redo: bool) {
    writer.buffers.harvest();
    let result = {
        let mut files = writer.files.write();
        let mut state = writer.buffers.model.write();
        if let Ok(model) = state.as_mut() {
            let grouped = files.as_mut().map_or(Ok(false), |p| {
                p.rename_history(model, redo).map_err(|e| e.to_string())
            });
            match grouped {
                Ok(true) => Ok(()),
                Ok(false) => {
                    if redo { model.redo() } else { model.undo() }.map_err(|e| e.to_string())
                }
                Err(e) => Err(e),
            }
        } else {
            Err("No document is open.".into())
        }
    };
    writer.buffers.sync(writer.dark);
    writer.message.report(result, String::new());
}
