//! Bounded reading previews: independent siblings, no recursive editor mounts.
use crate::design::Button;
use crate::design::tokens as t;
use crate::design::tokens::ProseTypography;
use crate::{editing::Writer, palette};
use freya::{
    animation::{OnCreation, use_animation},
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
            t::transition(0.3, 1., reduced)
        });
        let colors = t::colors();
        rect()
            .a11y_role(AccessibilityRole::Group)
            .a11y_alt("Destination preview · read only")
            .width(Size::fill())
            .padding((t::SPACE_SM, t::SPACE_MD))
            .spacing(t::SPACE_SM)
            .opacity(ink.get().value())
            .background(colors.inset)
            .border(
                Border::new()
                    .width(BorderWidth {
                        left: 2.,
                        ..Default::default()
                    })
                    .fill(colors.rule),
            )
            .child(
                label()
                    .text(format!("Preview · {}", palette::display_name(&id)))
                    .font_size(t::small())
                    .color(colors.muted),
            )
            .child(entries(&self.block.entries, writer.dark))
            .child(
                Button::new()
                    .flat()
                    .on_press(move |_| writer.inspect(&id))
                    .child(format!(
                        "{} {}",
                        if writer.localisation.read().active {
                            "Open"
                        } else {
                            "Edit"
                        },
                        palette::display_name(&self.block.id)
                    )),
            )
    }
}

pub(super) fn entries(items: &[ScriptEntry], dark: bool) -> Element {
    let mut budget = 32usize;
    let content = limited_entries(items, dark, &mut budget);
    rect()
        .width(Size::fill())
        .child(content)
        .maybe_child((budget == 0).then(|| {
            label()
                .text(crate::messages::text(
                    crate::messages::MsgId::WriterGuiPreviewLimit,
                ))
                .font_size(t::small())
        }))
        .into_element()
}
fn limited_entries(items: &[ScriptEntry], dark: bool, budget: &mut usize) -> Element {
    let mut result = rect().width(Size::fill()).spacing(t::SPACE_SM);
    for item in items {
        if *budget == 0 {
            break;
        }
        *budget -= 1;
        result = result.child(match item {
            ScriptEntry::Passage(passage) => rect()
                .width(Size::fill())
                .spacing(t::SPACE_XS)
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
                        .font_size(t::small())
                        .color(palette::muted(dark)),
                )
                .child(
                    label()
                        .text(passage.text.clone())
                        .prose_font()
                        .font_size(t::TEXT_EXCERPT),
                )
                .into_element(),
            ScriptEntry::Group {
                heading,
                entries: nested,
            } => rect()
                .width(Size::fill())
                .child(label().text(heading.clone()).font_size(t::small()))
                .child(limited_entries(nested, dark, budget))
                .into_element(),
            ScriptEntry::Jump(target) => label()
                .text(format!("→ {}", palette::display_name(target)))
                .font_size(t::small())
                .into_element(),
            ScriptEntry::Effect(text) | ScriptEntry::Source(text) => label()
                .text(text.clone())
                .font_size(t::small())
                .into_element(),
        });
    }
    result.into_element()
}
