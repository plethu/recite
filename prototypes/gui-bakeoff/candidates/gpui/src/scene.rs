use super::{Button, Command, Editor, Textarea, View, WorkbenchView};
use gpui::{prelude::*, *};
use gpui_component::Disableable;
use recite_bakeoff_authoring::PassageKind;

impl Render for WorkbenchView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let source = self.model.view() == &View::Source;
        let mut navigation = div().flex().flex_col().gap_2().w(px(200.)).flex_shrink_0();
        let mut scene = div()
            .id("scene")
            .flex()
            .flex_col()
            .gap_4()
            .flex_1()
            .min_w_0()
            .overflow_y_scroll();
        if source {
            scene = scene.child(
                Editor::new(&self.source)
                    .h(px(400.))
                    .aria_label("Recite source"),
            );
        }
        let mut section = String::new();
        match self.model.document().passages() {
            Err(error) => scene = scene.child(error.to_string()),
            Ok(passages) => {
                for passage in passages {
                    if passage.section != section {
                        section = passage.section.clone();
                        navigation = navigation
                            .child(div().mt_3().text_sm().child(section.replace('_', " ")));
                        if !source {
                            scene = scene.child(
                                div()
                                    .text_size(px(23.))
                                    .font_family("Noto Serif")
                                    .child(section.replace('_', " ")),
                            );
                        }
                    }
                    let choice = matches!(passage.kind, PassageKind::Choice { .. });
                    let caption = match &passage.kind {
                        PassageKind::Dialogue { speaker } => {
                            speaker.clone().unwrap_or_else(|| "Narration".into())
                        }
                        PassageKind::Choice { destination } => format!(
                            "Player choice · {}",
                            destination.as_deref().unwrap_or("next passage")
                        ),
                    };
                    let id = passage.id.clone();
                    navigation = navigation.child(
                        Button::new(SharedString::from(format!("nav-{id}")))
                            .label(if choice {
                                passage.text.chars().take(22).collect::<String>()
                            } else {
                                caption.replace('_', " ")
                            })
                            .on_click(cx.listener(move |s, _, window, cx| {
                                s.command(Command::Select(id.clone()), window, cx)
                            })),
                    );
                    if source {
                        continue;
                    }
                    let active = self.model.view() == &View::Passage(passage.id.clone());
                    let mut card = div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .p_4()
                        .rounded_lg()
                        .bg(rgb(match (self.dark, choice) {
                            (false, false) => 0xfffdfa,
                            (true, false) => 0x292a2d,
                            (false, true) => 0xf1d4bf,
                            (true, true) => 0x41332b,
                        }))
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(if self.dark { 0xe8e4de } else { 0x34322e }))
                                .child(caption.replace('_', " ")),
                        );
                    if active {
                        card = card
                            .child(self.attributes(cx))
                            .child(
                                div()
                                    .key_context("ReciteProse")
                                    .on_action(|_: &super::NextControl, window, cx| {
                                        window.focus_next(cx)
                                    })
                                    .on_action(|_: &super::PreviousControl, window, cx| {
                                        window.focus_prev(cx)
                                    })
                                    .child(
                                        Textarea::new(&self.prose)
                                            .h(px(110.))
                                            .font_family("Noto Serif")
                                            .text_color(rgb(if self.dark {
                                                0xe8e4de
                                            } else {
                                                0x34322e
                                            }))
                                            .text_size(px(20.))
                                            .aria_label("Passage text"),
                                    ),
                            )
                            .child(
                                Button::new("details")
                                    .label(if self.details {
                                        "Hide details"
                                    } else {
                                        "Details"
                                    })
                                    .on_click(cx.listener(|s, _, _, cx| {
                                        s.details = !s.details;
                                        cx.notify();
                                    })),
                            );
                        if self.details {
                            card = card.child(
                                div()
                                    .text_xs()
                                    .child(format!("{} @{}", passage.label, passage.id)),
                            );
                        }
                        card = card.child(
                            div()
                                .flex()
                                .gap_2()
                                .child(self.button("apply", "Apply draft", Command::Apply, cx))
                                .child(self.button(
                                    "discard",
                                    "Discard draft",
                                    Command::Discard,
                                    cx,
                                )),
                        );
                    } else {
                        let id = passage.id.clone();
                        card = card
                            .child(
                                div()
                                    .font_family("Noto Serif")
                                    .text_color(rgb(if self.dark { 0xe8e4de } else { 0x34322e }))
                                    .text_size(px(21.))
                                    .child(passage.text),
                            )
                            .child(
                                Button::new(SharedString::from(format!("edit-{id}")))
                                    .label("Edit")
                                    .on_click(cx.listener(move |s, _, window, cx| {
                                        s.command(Command::Select(id.clone()), window, cx)
                                    })),
                            );
                    }
                    scene = scene.child(card);
                }
            }
        }
        if source {
            scene = scene.child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.button("apply-source", "Apply draft", Command::Apply, cx))
                    .child(self.button("discard-source", "Discard draft", Command::Discard, cx)),
            );
        }
        scene = scene
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        self.button("add", "Add choice", Command::AddChoice, cx)
                            .disabled(source),
                    )
                    .child(self.button("try", "Try scene", Command::Preview, cx)),
            )
            .child(self.preview(cx));
        for diagnostic in self.model.document().diagnostics() {
            scene = scene.child(diagnostic.message.clone());
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .bg(rgb(if self.dark { 0x202124 } else { 0xf5f2ed }))
            .text_color(rgb(if self.dark { 0xe8e4de } else { 0x34322e }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(30.))
                            .font_family("Noto Serif")
                            .child("recite."),
                    )
                    .child(self.button("script", "Script", Command::Script, cx))
                    .child(self.button("source", "Source", Command::Source, cx))
                    .child(self.button("undo", "Undo", Command::Undo, cx))
                    .child(self.button("redo", "Redo", Command::Redo, cx))
                    .child(
                        Button::new("theme")
                            .label(if self.dark { "Light mode" } else { "Dark mode" })
                            .on_click(cx.listener(|s, _, window, cx| {
                                s.dark = !s.dark;
                                super::change_theme(s.dark, window, cx);
                                cx.notify();
                            })),
                    ),
            )
            .child(div().text_sm().child(if self.model.has_draft() {
                format!("Draft pending · {}", self.status)
            } else {
                self.status.clone()
            }))
            .child(
                div()
                    .flex()
                    .gap_6()
                    .flex_1()
                    .min_h_0()
                    .child(navigation)
                    .child(scene),
            )
    }
}
