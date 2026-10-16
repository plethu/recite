//! The graph and native specimen share this visual card, without sharing scene state.
use super::tokens as t;
use freya::prelude::*;

// Below this scale, the map becomes a structural overview instead of tiny prose.
pub(crate) const DIALOGUE_ZOOM: f32 = 0.6;

#[derive(Clone, PartialEq)]
pub(crate) struct BeatCard {
    pub title: String,
    pub speaker: String,
    pub excerpt: String,
    pub annotations: String,
    pub selected: bool,
    pub active: bool,
    pub zoom: f32,
}
impl Component for BeatCard {
    fn render(&self) -> impl IntoElement {
        let p = t::colors();
        let zoom = self.zoom;
        let mut content = rect()
            .width(Size::fill())
            .height(Size::fill())
            .padding(t::SPACE_MD * zoom)
            .spacing(t::SPACE_SM * zoom)
            .corner_radius(t::CARD_RADIUS * zoom)
            .shadow(super::material::offset(
                if self.selected { p.accent } else { p.teal },
                t::SHADOW_OFFSET * zoom,
            ))
            .background(if self.selected {
                Color::lerp(p.surface, p.selection, 0.35).into()
            } else {
                super::material::face(p.surface, 0.)
            })
            .border(Border::new().width(1.).fill(if self.selected {
                Color::lerp(p.accent, p.surface, 0.25)
            } else if self.active {
                p.boundary
            } else {
                p.rule
            }))
            .overflow(Overflow::Clip);
        if zoom >= 0.2 {
            let title = label()
                .text(self.title.clone())
                .font_size((16. * zoom).max(14.))
                .font_weight(FontWeight::SEMI_BOLD)
                .text_overflow(TextOverflow::Ellipsis)
                .max_lines(1);
            // The selected card's title sits on a plum dot screen, a printed header strip.
            // A fixed-height strip, so the dots stay behind the title and never reach the excerpt.
            content = content.child(
                rect()
                    .width(Size::fill())
                    .height(Size::px((16. * zoom).max(14.) * 1.45))
                    .main_align(Alignment::Center)
                    .maybe_child(self.selected.then(|| {
                        super::material::halftone(p.accent, t::HALFTONE_STRIP_ALPHA, 1., 5., false)
                    }))
                    .child(title),
            );
        }
        let mut passage = rect().width(Size::fill()).spacing(t::SPACE_XS * zoom);
        if zoom >= 0.4 {
            passage = passage.child(
                label()
                    .text(self.speaker.clone())
                    .font_size((t::small() * zoom).max(11.))
                    .max_lines(1)
                    .color(p.muted),
            );
        }
        if zoom >= DIALOGUE_ZOOM {
            passage = passage.child(
                t::prose().width(Size::fill()).child(
                    label()
                        .text(self.excerpt.clone())
                        .font_size((t::TEXT_EXCERPT * zoom).max(14.))
                        .line_height(t::EXCERPT_LINE_HEIGHT)
                        .text_overflow(TextOverflow::Ellipsis)
                        .max_lines(if zoom < 0.8 { 2 } else { 3 }),
                ),
            );
        }
        if zoom >= 0.4 {
            content = content.child(passage);
        }
        if zoom >= 0.4 && !self.annotations.is_empty() {
            content = content.child(
                label()
                    .text(self.annotations.clone())
                    .font_size((t::small() * zoom).max(11.))
                    .max_lines(1)
                    .color(p.muted),
            );
        }
        content
    }
}
