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
        let mut available = writer.layout.available;
        let navigation_width = writer.layout.drawer_width;
        let script_width = writer.layout.script_width;
        let focus = *writer.layout.focus.read();
        let standalone = writer.layout.standalone();
        let visible = writer.layout.navigation_expanded();
        let show_split = writer.layout.show_split(writer);
        let full_script =
            !self.source && (standalone || *writer.pane.read() == Pane::Script) && !show_split;
        let editing = !self.source && *writer.pane.read() == Pane::Script && show_split;
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
            t::collapsed_drawer_width()
        };
        let script_max = SCRIPT_MAX.min(
            (*available.read() - nav_width - MAP_MIN - 2. * t::SPLITTER_WIDTH).max(SCRIPT_MIN),
        );
        let pane_width = (*script_width.read()).clamp(SCRIPT_MIN, script_max);
        let pane = self.writing_pane(if full_script {
            f32::INFINITY
        } else {
            pane_width
        });
        let divider = Splitter {
            name: crate::messages::text(crate::messages::MsgId::WriterWorkspaceResizeScript),
            width: script_width,
            min: SCRIPT_MIN,
            max: script_max,
            direction: if left { 1. } else { -1. },
            changed: EventHandler::new(move |width| {
                crate::presentation::resize(
                    writer,
                    recite_config::WriterPresentationField::ScriptWidth,
                    width,
                )
            }),
        };
        let mut editor = rect()
            .horizontal()
            .content(Content::Flex)
            .width(Size::fill())
            .height(Size::flex(1.));
        if *writer.pane.read() == Pane::Disk {
            editor = editor.child(crate::external::ExternalScreen { writer });
        } else if *writer.pane.read() == Pane::Rename {
            editor = editor.child(crate::rename::RenameScreen { writer });
        } else if *writer.pane.read() == Pane::Build {
            editor = editor.child(crate::builds::BuildScreen { writer });
        } else if *writer.pane.read() == Pane::Declarations {
            editor = editor.child(crate::declarations::Declarations { writer });
        } else if *writer.pane.read() == Pane::Rules {
            editor = editor.child(crate::rules::RulesScreen { writer });
        } else if *writer.pane.read() == Pane::Preview {
            editor = editor.child(crate::preview_panel::PreviewScreen { writer });
        } else if writer.localisation.read().active {
            editor = editor.child(crate::localisation::Surface {
                writer,
                reading: self.reading.clone(),
                files: self.files,
            });
        } else if self.source || full_script {
            editor = editor.child(pane);
        } else if editing && left {
            editor = editor.child(pane).child(divider).child(self.map.clone());
        } else {
            editor = editor.child(self.map.clone());
            if editing {
                editor = editor.child(divider).child(pane);
            }
        }
        let updates = writer.localisation.read().active
            && writer.localisation.read().view == crate::localisation::CatalogueView::Updates;
        rect()
            .width(Size::flex(1.))
            .height(Size::fill())
            .horizontal()
            .content(Content::Flex)
            .on_sized(move |e: Event<SizedEventData>| {
                available.set_if_modified(e.area.width());
            })
            .maybe_child((!updates && !focus).then_some(Sidebar {
                writer,
                visible: self.navigation_visible,
                width: nav_width,
                expanded: visible,
                scenes: self.scenes.clone(),
            }))
            .maybe_child((visible && !updates).then_some(Splitter {
                name: crate::messages::text(crate::messages::MsgId::WriterWorkspaceResizeDrawer),
                width: navigation_width,
                min: NAV_MIN,
                max: nav_max,
                direction: 1.,
                changed: EventHandler::new(move |width| {
                    crate::presentation::resize(
                        writer,
                        recite_config::WriterPresentationField::DrawerWidth,
                        width,
                    )
                }),
            }))
            .child(
                rect()
                    .key("scene-editor")
                    .content(Content::Flex)
                    .width(Size::flex(1.))
                    .height(Size::fill())
                    .background(palette::reading(writer.dark))
                    .maybe_child((!focus).then_some(crate::document_tabs::DocumentTabs { writer }))
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
            .on_pointer_down(move |_| {
                writer.vim.enter(writer.inspector_focus);
                if !source {
                    writer.inspector_focus.request_focus();
                }
            })
            .on_sized(move |e: Event<SizedEventData>| {
                writer.vim.area(writer.inspector_focus, e.area)
            })
            .content(Content::Flex)
            .a11y_id(writer.inspector_focus)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Group)
            .a11y_alt("Beat editor")
            .width(if source || !width.is_finite() {
                Size::fill()
            } else {
                Size::px(width)
            })
            .height(Size::fill())
            .on_key_down(move |e: Event<KeyboardEventData>| {
                writer.vim.enter(writer.inspector_focus);
                if e.key == Key::Named(NamedKey::Escape) && !source && !writer.layout.standalone() {
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
                    .child(
                        rect()
                            .width(Size::fill())
                            .cross_align(Alignment::Center)
                            .padding((t::SPACE_MD, t::SPACE_LG))
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .max_width(Size::px(t::prose_size() * 34.))
                                    .child(self.reading.clone()),
                            ),
                    )
                    .into_element()
            })
            .maybe_child((!source).then_some(crate::reading_context::ReferencePanel { writer }))
            .into_element()
    }
}
