use super::{Button, Command, WorkbenchView};
use gpui::{prelude::*, *};
use gpui_component::Disableable;
use recite_bakeoff_authoring::PassageKind;

impl WorkbenchView {
    pub(super) fn attributes(&self, cx: &Context<Self>) -> Div {
        let mut row = div().flex().flex_wrap().gap_2();
        if let Ok(Some(passage)) = self.model.selected() {
            let values = match passage.kind {
                PassageKind::Dialogue { .. } => vec!["alice".into(), "cheshire_cat".into()],
                PassageKind::Choice { .. } => {
                    let mut sections = self.model.document().sections();
                    sections.push("END".into());
                    sections
                }
            };
            for value in values {
                row = row.child(
                    Button::new(SharedString::from(format!("attribute-{value}")))
                        .label(value.replace('_', " "))
                        .on_click(cx.listener(move |s, _, window, cx| {
                            s.command(Command::Attribute(value.clone()), window, cx)
                        })),
                );
            }
        }
        row
    }

    pub(super) fn preview(&self, cx: &Context<Self>) -> Div {
        let mut panel = div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .bg(rgb(if self.dark { 0x39352b } else { 0xf1e6cc }))
            .rounded_lg();
        if let Some(page) = self.model.preview_page() {
            panel = panel
                .child(if self.model.preview_stale() {
                    "Preview out of date — apply or discard your draft, then try the scene again."
                } else {
                    "Trying the scene"
                })
                .child(page.text.clone());
            for (index, choice) in page.choices.iter().enumerate() {
                panel = panel.child(
                    Button::new(SharedString::from(format!("preview-{index}")))
                        .label(choice.text.clone())
                        .disabled(self.model.preview_stale())
                        .on_click(cx.listener(move |s, _, window, cx| {
                            s.command(Command::Advance(Some(index)), window, cx)
                        })),
                );
            }
            if page.ended {
                panel = panel.child("Scene ended");
            } else if page.choices.is_empty() {
                panel = panel.child(
                    self.button("continue", "Continue", Command::Advance(None), cx)
                        .disabled(self.model.preview_stale()),
                );
            }
        } else {
            panel = panel.child("Try the scene to read through the conversation and its choices.");
        }
        panel
    }
}
