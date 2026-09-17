//! The navigation, map and script share one bounded split layout.
use crate::{
    design::{Splitter, tokens as t},
    editing::{Pane, Writer},
    palette,
    sidebar::Sidebar,
};
use freya::prelude::*;
use recite_config::WriterPaneSide;

const NAV_MIN: f32 = 180.;
const NAV_MAX: f32 = 360.;
const SCRIPT_MIN: f32 = 320.;
const SCRIPT_MAX: f32 = 800.;
const MAP_MIN: f32 = 320.;

#[derive(Clone)]
pub(super) struct Workspace {
    pub writer: Writer,
    pub files: State<Option<crate::project::ProjectFiles>>,
    pub source: bool,
    pub navigation_visible: State<bool>,
    pub scenes: Element,
    pub toolbar: Element,
    pub map: Element,
    pub reading: Element,
    pub active: Element,
}
impl PartialEq for Workspace {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
            && self.source == other.source
            && self.scenes == other.scenes
            && self.toolbar == other.toolbar
            && self.map == other.map
            && self.reading == other.reading
            && self.active == other.active
    }
}
impl Component for Workspace {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let mut available = use_state(|| 1200.);
        let navigation_width = use_state(|| 224_f32);
        let script_width = use_state(|| 520_f32);
        let visible = *self.navigation_visible.read();
        let editing = !self.source && *writer.pane.read() == Pane::Script;
        let left = writer.preferences.read().config.writer.pane_side == WriterPaneSide::Left;
        let nav_max = NAV_MAX.min(
            (*available.read()
                - MAP_MIN
                - if editing {
                    SCRIPT_MIN + 2. * t::SPLITTER_WIDTH
                } else {
                    t::SPLITTER_WIDTH
                })
            .max(NAV_MIN),
        );
        let nav_width = if visible {
            (*navigation_width.read()).clamp(NAV_MIN, nav_max)
        } else {
            t::COLLAPSED_DRAWER_WIDTH
        };
        let script_max = SCRIPT_MAX.min(
            (*available.read() - nav_width - MAP_MIN - 2. * t::SPLITTER_WIDTH).max(SCRIPT_MIN),
        );
        let pane_width = (*script_width.read()).clamp(SCRIPT_MIN, script_max);
        let pane = self.writing_pane(pane_width);
        let divider = Splitter {
            name: "Resize script pane",
            width: script_width,
            min: SCRIPT_MIN,
            max: script_max,
            direction: if left { 1. } else { -1. },
        };
        let mut editor = rect()
            .horizontal()
            .content(Content::Flex)
            .width(Size::fill())
            .height(Size::flex(1.));
        if writer.localisation.read().active {
            editor = editor.child(crate::localisation::Surface {
                writer,
                reading: self.reading.clone(),
                files: self.files,
            });
        } else if self.source {
            editor = editor.child(pane);
        } else if editing && left {
            editor = editor.child(pane).child(divider).child(self.map.clone());
        } else {
            editor = editor.child(self.map.clone());
            if editing {
                editor = editor.child(divider).child(pane);
            }
        }
        rect()
            .width(Size::flex(1.))
            .height(Size::fill())
            .horizontal()
            .content(Content::Flex)
            .on_sized(move |e: Event<SizedEventData>| {
                available.set_if_modified(e.area.width());
            })
            .child(Sidebar {
                writer,
                visible: self.navigation_visible,
                width: nav_width,
                scenes: self.scenes.clone(),
            })
            .maybe_child(visible.then_some(Splitter {
                name: "Resize scene drawer",
                width: navigation_width,
                min: NAV_MIN,
                max: nav_max,
                direction: 1.,
            }))
            .child(
                rect()
                    .key("scene-editor")
                    .content(Content::Flex)
                    .width(Size::flex(1.))
                    .height(Size::fill())
                    .background(palette::reading(writer.dark))
                    .child(self.toolbar.clone())
                    .child(editor),
            )
    }
}
impl Workspace {
    fn writing_pane(&self, width: f32) -> Element {
        let writer = self.writer;
        let source = self.source;
        rect()
            .key("writing-pane")
            .content(Content::Flex)
            .a11y_id(writer.inspector_focus)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Group)
            .a11y_alt("Beat editor")
            .width(if source {
                Size::fill()
            } else {
                Size::px(width)
            })
            .height(Size::fill())
            .on_key_down(move |e: Event<KeyboardEventData>| {
                if e.key == Key::Named(NamedKey::Escape) && !source {
                    e.stop_propagation();
                    writer.close_editor();
                }
            })
            .child(if source {
                self.active.clone()
            } else {
                ScrollView::new_controlled(writer.scroll)
                    .height(Size::flex(1.))
                    .width(Size::fill())
                    .child(self.reading.clone())
                    .into_element()
            })
            .maybe_child((!source).then_some(crate::reading_context::ReferencePanel { writer }))
            .into_element()
    }
}
