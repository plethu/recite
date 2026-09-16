//! Bounded reading previews: independent siblings, no recursive editor mounts.
use crate::{editing::Writer, palette};
use freya::{
    animation::{AnimNum, OnCreation, use_animation},
    prelude::*,
};
use recite_writer_model::{PassageKind, ScriptBlock, ScriptEntry};

#[derive(Clone)]
pub(super) struct BranchPreview {
    pub writer: Writer,
    pub block: ScriptBlock,
}
impl PartialEq for BranchPreview {
    fn eq(&self, other: &Self) -> bool {
        self.block == other.block && self.writer.dark == other.writer.dark
    }
}
impl Component for BranchPreview {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let id = self.block.id.clone();
        let reduced = writer.preferences.read().config.writer.reduced_motion;
        let ink = use_animation(move |config| {
            config.on_creation(OnCreation::Run);
            AnimNum::new(0.3, 1.).time(if reduced { 0 } else { 180 })
        });
        rect()
            .width(Size::fill())
            .padding((8., 16.))
            .spacing(8.)
            .opacity(ink.get().value())
            .border(
                Border::new()
                    .width(BorderWidth {
                        left: 2.,
                        ..Default::default()
                    })
                    .fill(palette::rule(writer.dark)),
            )
            .child(label().text(palette::display_name(&id)).font_size(16.))
            .child(entries(&self.block.entries, writer.dark))
            .child(
                Button::new()
                    .flat()
                    .compact()
                    .on_press(move |_| writer.inspect(&id))
                    .child(format!("Edit {}", palette::display_name(&self.block.id))),
            )
    }
}

fn entries(items: &[ScriptEntry], dark: bool) -> Element {
    let mut result = rect().width(Size::fill()).spacing(8.);
    for item in items {
        result = result.child(match item {
            ScriptEntry::Passage(passage) => rect()
                .width(Size::fill())
                .spacing(3.)
                .child(
                    label()
                        .text(match &passage.kind {
                            PassageKind::Dialogue { speaker } => speaker
                                .as_deref()
                                .map(palette::display_name)
                                .unwrap_or_else(|| "Dialogue".into()),
                            PassageKind::Choice { destination } => destination
                                .as_deref()
                                .map(|target| format!("Reply → {}", palette::display_name(target)))
                                .unwrap_or_else(|| "Reply".into()),
                        })
                        .font_size(12.)
                        .color(palette::muted(dark)),
                )
                .child(
                    label()
                        .text(passage.text.clone())
                        .font_family("serif")
                        .font_size(17.),
                )
                .into_element(),
            ScriptEntry::Group {
                heading,
                entries: nested,
            } => rect()
                .width(Size::fill())
                .child(label().text(heading.clone()).font_size(12.))
                .child(entries(nested, dark))
                .into_element(),
            ScriptEntry::Jump(target) => label()
                .text(format!("→ {}", palette::display_name(target)))
                .font_size(12.)
                .into_element(),
            ScriptEntry::Effect(text) | ScriptEntry::Source(text) => {
                label().text(text.clone()).font_size(12.).into_element()
            }
        });
    }
    result.into_element()
}
