//! A deliberate runtime trial. Requests are answered by the author, never the game.
mod controls;
mod prepare;
mod snapshot;
mod trace;
use crate::design::tokens::ProseTypography;
use crate::messages::{MsgId, text};
use crate::{
    design::{Button, tokens as t},
    editing::{Pane, Writer},
};
use freya::prelude::*;
use recite_writer_model::{ConditionExpectedType, ConditionValue, EffectAck};
pub(crate) use snapshot::Snapshot;

#[derive(Clone)]
pub(super) struct PreviewScreen {
    pub writer: Writer,
}
impl PartialEq for PreviewScreen {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
    }
}
impl Component for PreviewScreen {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let mut pane = writer.pane;
        let mut trace_open = use_state(|| false);
        let answer = use_state(String::new);
        let state = writer.buffers.model.read();
        let session = state.as_ref().ok();
        let page = session.and_then(|session| session.preview_page());
        let mut panel = rect()
            .width(Size::fill())
            .padding(t::SPACE_XL)
            .spacing(t::SPACE_LG)
            .child(
                label()
                    .text(text(MsgId::WriterPreview))
                    .font_size(t::title()),
            )
            .child(
                crate::design::actions().child(
                    Button::new()
                        .flat()
                        .on_press(move |_| pane.set(Pane::Script))
                        .child(text(MsgId::WriterClosePreview)),
                ),
            );
        if writer.localisation.read().entry_context.is_some() {
            panel = panel.child(
                Button::new()
                    .flat()
                    .on_press(move |_| {
                        writer.localisation.write().active = true;
                        writer.localisation.write().view =
                            crate::localisation::CatalogueView::Entry;
                        writer.pane.set(Pane::Script);
                    })
                    .child(text(MsgId::WriterReturnTranslation)),
            );
        }
        panel = panel.child(controls::Controls { writer });
        if let Some(caption) = writer.trial.snapshot.read().as_ref() {
            panel = panel.child(label().text(caption.caption.clone()).font_size(t::small()));
        }
        if session.is_some_and(|s| s.preview_stale()) || writer.trial.catalogue_stale(writer) {
            panel = panel.child(label().text(text(MsgId::WriterPreviewStale)));
        }
        let Some(page) = page else {
            return ScrollView::new()
                .width(Size::fill())
                .height(Size::flex(1.))
                .child(panel)
                .into_element();
        };
        panel = panel.child(
            label()
                .text(page.text.clone())
                .prose_font()
                .font_size(t::prose_size()),
        );
        if let Some(request) = &page.condition {
            panel = panel
                .child(label().text(text(MsgId::WriterConditionInput)))
                .child(label().text(trace::query(request.query())));
            if request.query().expected_type() == ConditionExpectedType::Bool {
                let mut choices = crate::design::actions();
                for (value, caption) in [(true, MsgId::WriterTrue), (false, MsgId::WriterFalse)] {
                    choices = choices.child(
                        Button::new()
                            .on_press(move |_| {
                                writer.perform(|m| m.answer_preview(ConditionValue::Bool(value)))
                            })
                            .child(text(caption)),
                    );
                }
                panel = panel.child(choices);
            } else {
                panel = panel.child(Input::new(answer).width(Size::fill())).child(
                    Button::new()
                        .enabled(!answer.read().trim().is_empty())
                        .on_press(move |_| {
                            writer.perform(|m| {
                                m.answer_preview(ConditionValue::EnumVariant(
                                    answer.peek().trim().to_owned(),
                                ))
                            });
                        })
                        .child(text(MsgId::WriterConditionAnswer)),
                );
            }
        }
        for effect in &page.effects {
            panel = panel.child(label().text(trace::effect(effect)).font_size(t::small()));
        }
        if page.waiting_effect.is_some() {
            panel = panel
                .child(label().text(text(MsgId::WriterAwaitingEffect)))
                .child(
                    crate::design::actions()
                        .child(
                            Button::new()
                                .filled()
                                .on_press(move |_| {
                                    writer.perform(|m| m.acknowledge_preview(EffectAck::Completed))
                                })
                                .child(text(MsgId::WriterAcknowledge)),
                        )
                        .child(
                            Button::new()
                                .on_press(move |_| {
                                    writer.perform(|m| {
                                        m.acknowledge_preview(EffectAck::Failed {
                                            reason: text(MsgId::WriterSampleFailure),
                                        })
                                    })
                                })
                                .child(text(MsgId::WriterEffectFailed)),
                        ),
                );
        }
        for (index, choice) in page.choices.iter().enumerate() {
            panel = panel.child(
                Button::new()
                    .on_press(move |_| writer.perform(|m| m.advance_preview(Some(index))))
                    .child(choice.text.clone()),
            );
        }
        if page.choices.is_empty()
            && !page.ended
            && page.condition.is_none()
            && page.waiting_effect.is_none()
        {
            panel = panel.child(
                Button::new()
                    .filled()
                    .on_press(move |_| writer.perform(|m| m.advance_preview(None)))
                    .child(text(MsgId::WriterContinue)),
            );
        }
        panel = panel.child(
            Button::new()
                .flat()
                .on_press(move |_| {
                    let next = !*trace_open.peek();
                    trace_open.set(next);
                })
                .child(text(MsgId::WriterPreviewTrace)),
        );
        if *trace_open.read()
            && let Some(trace) = session.and_then(|s| s.preview_trace())
        {
            panel = panel.child(trace::render(trace));
        }
        ScrollView::new()
            .width(Size::fill())
            .height(Size::flex(1.))
            .child(panel)
            .into_element()
    }
}

/// Staged controls live with the workspace, independently of the active run.
#[derive(Clone, Copy)]
pub(crate) struct TrialInputs {
    pub locale: State<String>,
    pub variant: State<String>,
    pub include_drafts: State<bool>,
    pub values: State<std::collections::BTreeMap<String, String>>,
    pub snapshot: State<Option<Snapshot>>,
}
impl TrialInputs {
    pub(crate) fn catalogue_stale(&self, writer: Writer) -> bool {
        self.snapshot
            .read()
            .as_ref()
            .is_some_and(|snapshot| snapshot.stale(writer.localisation.read().catalogue.as_ref()))
    }
    pub fn new() -> Self {
        Self {
            snapshot: use_state(|| None),
            locale: use_state(String::new),
            variant: use_state(String::new),
            include_drafts: use_state(|| false),
            values: use_state(std::collections::BTreeMap::new),
        }
    }
}
