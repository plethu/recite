//! A stable navigation rail with an animated scene outline and fixed settings footer.
use crate::design::tokens as t;
use crate::{controls, editing::Writer};
use freya::prelude::*;

#[derive(Clone)]
pub(super) struct Sidebar {
    pub writer: Writer,
    pub visible: State<bool>,
    pub width: f32,
    pub scenes: Element,
}
impl PartialEq for Sidebar {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
            && self.width == other.width
            && self.scenes == other.scenes
    }
}
impl Component for Sidebar {
    fn render(&self) -> impl IntoElement {
        render(self.writer, self.visible, self.width, self.scenes.clone())
    }
}
fn render(
    mut writer: Writer,
    mut navigation_visible: State<bool>,
    width: f32,
    scenes: Element,
) -> Element {
    let visible = *navigation_visible.read();
    let colors = use_theme().read().colors.clone();
    let body = rect()
        .height(Size::fill())
        .content(Content::Flex)
        .width(Size::fill())
        .padding((4., 12.))
        .spacing(t::SPACE_XS)
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt("Scenes and beats")
        .child(
            label()
                .text("Scenes")
                .font_size(t::TEXT_SMALL)
                .color(colors.text_secondary),
        )
        .child(scenes);
    rect()
        .a11y_id(writer.sidebar_focus)
        .a11y_focusable(true)
        .a11y_role(AccessibilityRole::Navigation)
        .a11y_alt("Scene navigation")
        .width(Size::px(if visible {
            width
        } else {
            t::COLLAPSED_DRAWER_WIDTH
        }))
        .height(Size::fill())
        .content(Content::Flex)
        .overflow(Overflow::Clip)
        .child(
            rect()
                .height(Size::px(48.))
                .width(Size::fill())
                .maybe_child(visible.then(|| {
                    rect()
                        .padding((8., 12.))
                        .child(label().text("recite.").font_size(t::TEXT_TITLE))
                }))
                .child(
                    rect()
                        .position(Position::new_absolute().right(4.).top(8.))
                        .child(controls::IconButton::new(
                            if visible {
                                "Hide scenes"
                            } else {
                                "Show scenes"
                            },
                            controls::Icon::Sidebar,
                            move || {
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
                .height(Size::px(48.))
                .width(Size::fill())
                .padding((8., 4.))
                .child(controls::IconButton::new(
                    "Settings",
                    controls::Icon::Settings,
                    move || writer.settings_open.set(true),
                )),
        )
        .into_element()
}
