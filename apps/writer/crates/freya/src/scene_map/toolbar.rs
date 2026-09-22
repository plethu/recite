//! Map actions reflow to the available pane width before labels are squeezed.
use super::{
    camera::{Camera, Framing},
    placement::Arrangement,
};
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    palette,
};
use freya::prelude::*;

pub(super) struct Toolbar {
    pub writer: Writer,
    pub scene: String,
    pub camera: State<Camera>,
    pub viewport: State<(f32, f32)>,
    pub positions: State<Arrangement>,
    pub bounds: (f32, f32, f32, f32),
    pub show_help: State<bool>,
}
impl Toolbar {
    pub fn render(self) -> Element {
        let Self {
            mut writer,
            scene,
            mut camera,
            viewport,
            mut positions,
            bounds,
            mut show_help,
        } = self;
        let mut pane = writer.pane;
        let compact = viewport.read().0 < 480. * t::ui_scale();
        let primary_actions = rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t::SPACE_SM)
            .child(
                Button::new()
                    .flat()
                    .enabled(writer.selection.read().is_some())
                    .on_press(move |_| {
                        let selected = writer.selection.peek().clone();
                        if let Some(id) = selected {
                            writer.inspect(&id);
                        }
                    })
                    .child(crate::messages::text(
                        crate::messages::MsgId::WriterWorkspaceOpenScript,
                    )),
            )
            .child(
                Button::new()
                    .flat()
                    .on_press(move |_| {
                        writer.navigate(recite_writer_model::Workbench::add_beat);
                        pane.set(crate::editing::Pane::Script);
                    })
                    .child(crate::messages::text(
                        crate::messages::MsgId::WriterGuiAddBeat,
                    )),
            );
        let secondary_actions = rect()
            .horizontal()
            .spacing(t::SPACE_SM)
            .child(
                Button::new()
                    .flat()
                    .on_press(move |_| {
                        positions.write().reset(&scene);
                        camera.write().framing = Framing::Free;
                    })
                    .named(crate::messages::text(
                        crate::messages::MsgId::WriterGuiArrangeAutomatically,
                    ))
                    .child(crate::messages::text(
                        crate::messages::MsgId::WriterGuiArrange,
                    )),
            )
            .child(
                Button::new()
                    .flat()
                    .named(crate::messages::text(
                        crate::messages::MsgId::WriterGuiMapHelp,
                    ))
                    .expanded(*show_help.read())
                    .on_press(move |_| {
                        let next = !*show_help.peek();
                        show_help.set(next);
                    })
                    .child(crate::messages::text(crate::messages::MsgId::WriterGuiHelp)),
            );
        let actions = (if compact { rect() } else { rect().horizontal() })
            .spacing(t::SPACE_SM)
            .child(primary_actions)
            .child(secondary_actions);
        let search = rect()
            .width(if compact {
                Size::fill()
            } else {
                Size::flex(1.)
            })
            .max_width(Size::px(320. * t::ui_scale()))
            .child(
                Input::new(writer.search)
                    .a11y_id(writer.search_focus)
                    .width(Size::fill())
                    .height(Size::px(t::control_height()))
                    .placeholder("Find a beat…")
                    .on_pre_key_down(move |event: Event<KeyboardEventData>| {
                        if event.code == Code::Escape {
                            writer.map_focus.request_focus();
                            event.stop_propagation();
                            return false;
                        }
                        if event.code == Code::Enter {
                            let query = writer.search.peek().to_lowercase();
                            if let Ok(m) = writer.buffers.model.peek().as_ref()
                                && let Ok(blocks) = m.document().script()
                                && let Some(block) = blocks.iter().find(|b| {
                                    palette::display_name(&b.id).to_lowercase().contains(&query)
                                })
                            {
                                writer.selection.set(Some(block.id.clone()));
                                writer.map_focus.request_focus();
                            }
                            event.stop_propagation();
                            return false;
                        }
                        crate::closing::text_input_key(event)
                    }),
            );
        let colors = crate::design::palette::Palette::new(writer.dark);
        let zoom_controls = rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t::SPACE_XS)
            .a11y_role(AccessibilityRole::Group)
            .a11y_alt("Map zoom")
            .corner_radius(t::RADIUS)
            .background(colors.inset)
            .border(Border::new().width(1.).fill(colors.rule))
            .child(crate::controls::IconButton::new(
                "Zoom out",
                crate::controls::Icon::ZoomOut,
                move || {
                    let zoom = camera.peek().zoom / 1.2;
                    camera.write().zoom_to(zoom, *viewport.peek());
                },
            ))
            .child(
                Button::new()
                    .flat()
                    .named(crate::messages::text(
                        crate::messages::MsgId::WriterGuiResetZoom,
                    ))
                    .on_press(move |_| {
                        camera.write().zoom_to(1., *viewport.peek());
                    })
                    .child(format!("{:.0}%", camera.read().zoom * 100.)),
            )
            .child(crate::controls::IconButton::new(
                "Zoom in",
                crate::controls::Icon::ZoomIn,
                move || {
                    let zoom = camera.peek().zoom * 1.2;
                    camera.write().zoom_to(zoom, *viewport.peek());
                },
            ))
            .child(
                Button::new()
                    .flat()
                    .on_press(move |_| {
                        camera.write().fit(*viewport.peek(), bounds);
                    })
                    .child(crate::messages::text(crate::messages::MsgId::WriterGuiFit)),
            );
        let navigation = (if compact { rect() } else { rect().horizontal() })
            .width(Size::fill())
            .content(Content::Flex)
            .spacing(t::SPACE_SM)
            .child(search)
            .child(zoom_controls);
        rect()
            .width(Size::fill())
            .spacing(t::SPACE_SM)
            .child(actions)
            .child(navigation)
            .into_element()
    }
}
