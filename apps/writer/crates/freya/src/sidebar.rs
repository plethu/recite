//! A stable navigation rail with an animated scene outline and fixed settings footer.
use crate::commands::CommandExt;
use crate::design::tokens as t;
use crate::{controls, editing::Writer};
use freya::prelude::*;

#[derive(Clone)]
pub(super) struct Sidebar {
    pub writer: Writer,
    pub visible: State<bool>,
    pub width: f32,
    pub expanded: bool,
    pub scenes: Element,
}
impl PartialEq for Sidebar {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
            && self.expanded == other.expanded
            && self.width == other.width
            && self.scenes == other.scenes
    }
}
impl Component for Sidebar {
    fn render(&self) -> impl IntoElement {
        render(
            self.writer,
            self.visible,
            self.width,
            self.expanded,
            self.scenes.clone(),
        )
    }
}
fn render(
    writer: Writer,
    mut navigation_visible: State<bool>,
    width: f32,
    visible: bool,
    scenes: Element,
) -> Element {
    let constrained = *writer.layout.available.read() < 900. * t::ui_scale();
    let body = rect()
        .on_sized(move |e: Event<SizedEventData>| writer.vim.area(writer.sidebar_focus, e.area))
        .on_key_down(move |_| writer.vim.enter(writer.sidebar_focus))
        .height(Size::fill())
        .content(Content::Flex)
        .width(Size::fill())
        // Horizontal padding lives in each child, so selected rows can bleed to the rail edge.
        .padding((4., 0.))
        .spacing(t::SPACE_XS)
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt("Scenes and beats")
        .child(
            // A running head: the caps label over a short plum rule.
            rect()
                .padding((t::SPACE_XS, t::RAIL_PADDING))
                .spacing(t::SPACE_XS)
                .child(t::caps(&crate::messages::text(
                    crate::messages::MsgId::WriterGuiScenes,
                )))
                .child(
                    rect()
                        .width(Size::px(24.))
                        .height(Size::px(t::STROKE_WIDTH))
                        .background(t::colors().accent),
                ),
        )
        .child(scenes);
    rect()
        .on_pointer_down(move |_| {
            writer.vim.enter(writer.sidebar_focus);
            writer.sidebar_focus.request_focus();
        })
        .on_key_down(move |_| writer.vim.enter(writer.sidebar_focus))
        .a11y_id(writer.sidebar_focus)
        .a11y_focusable(true)
        .a11y_role(AccessibilityRole::Navigation)
        .a11y_alt("Scene navigation")
        .width(Size::px(if visible {
            width
        } else {
            t::collapsed_drawer_width()
        }))
        .height(Size::fill())
        .content(Content::Flex)
        .overflow(Overflow::Clip)
        .background(t::colors().canvas)
        .child(crate::design::material::grain(t::colors().ink))
        .child(
            rect()
                .height(Size::px(t::control_height() + 16.))
                .width(Size::fill())
                .maybe_child(visible.then(|| {
                    rect().padding((8., 12.)).child(
                        SvgViewer::new(Bytes::from_static(include_bytes!(
                            "../../../../../assets/identity/recite-wordmark.svg"
                        )))
                        .width(Size::px(27. * t::WORDMARK_ASPECT * t::ui_scale()))
                        .height(Size::px(27. * t::ui_scale()))
                        .fill(t::colors().wordmark)
                        .color(t::colors().teal)
                        .a11y_alt("Recite"),
                    )
                }))
                .child(
                    rect()
                        .position(Position::new_absolute().right(4.).top(8.))
                        .child(controls::IconButton::new(
                            if constrained {
                                "Go to scene or beat"
                            } else if visible {
                                "Hide scenes"
                            } else {
                                "Show scenes"
                            },
                            controls::Icon::Sidebar,
                            move || {
                                if constrained {
                                    crate::commands::Command::GoTo.run(writer);
                                    return;
                                }
                                let next = !*navigation_visible.peek();
                                navigation_visible.set(next);
                            },
                        )),
                ),
        )
        .child(
            rect()
                .height(Size::flex(1.))
                .width(Size::fill())
                .maybe_child(visible.then(|| crate::design::Reveal {
                    reduced_motion: writer.preferences.read().config.writer.reduced_motion,
                    content: body.into_element(),
                })),
        )
        .child(
            rect()
                .height(Size::px(t::control_height() + 16.))
                .width(Size::fill())
                .padding((8., 4.))
                .child(controls::IconButton::new(
                    "Settings",
                    controls::Icon::Settings,
                    move || crate::commands::Command::Settings.run(writer),
                )),
        )
        .into_element()
}
