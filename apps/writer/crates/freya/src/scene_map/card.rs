//! Beat cards keep dialogue, speaker and structural annotations distinct.
use super::{
    arrangement::Handle,
    camera::Camera,
    layout::{HEIGHT, Node},
    placement::{Arrangement, card_key},
};
use crate::{editing::Writer, palette};
use freya::prelude::*;
use recite_writer_model::ScriptBlock;
use recite_writer_session::map::synopsis::synopsis;

pub(super) use crate::design::DIALOGUE_ZOOM;

#[derive(Clone)]
pub(super) struct Card {
    pub writer: Writer,
    pub block: ScriptBlock,
    pub node: Node,
    pub selected: bool,
    pub related: bool,
    pub ending: bool,
    pub scene: String,
    pub hovered: State<Option<String>>,
    pub positions: State<Arrangement>,
    pub camera: State<Camera>,
    pub zoom: f32,
    pub pan: State<Option<(f64, f64)>>,
}
impl PartialEq for Card {
    fn eq(&self, other: &Self) -> bool {
        self.zoom == other.zoom
            && self.block == other.block
            && self.node == other.node
            && self.selected == other.selected
            && self.related == other.related
            && self.ending == other.ending
            && self.scene == other.scene
            && self.writer.dark == other.writer.dark
    }
}
impl Component for Card {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let zoom = self.zoom;
        let mut hovered = self.hovered;
        let pan = self.pan;
        let id = self.block.id.clone();
        let enter_id = id.clone();
        let focus = use_a11y();
        let focus_id = id.clone();
        let mut had_focus = use_state(|| false);
        use_side_effect(move || {
            let focused = focus.is_focused();
            if focused && !*had_focus.peek() {
                hovered.set(Some(focus_id.clone()));
            } else if !focused && *had_focus.peek() && hovered.peek().as_ref() == Some(&focus_id) {
                hovered.set(None);
            }
            had_focus.set_if_modified(focused);
        });
        let active = self.related || hovered.read().as_ref() == Some(&id) || focus.is_focused();
        let synopsis = synopsis(&self.block.entries);
        let description = format!(
            "{}. {}. {}",
            synopsis.speaker,
            synopsis.prose,
            synopsis.annotations.join(", ")
        );
        let mut title = palette::display_name(&id);
        if self.ending {
            title.push_str(" · end");
        }
        let content = crate::design::BeatCard {
            title: title.clone(),
            speaker: synopsis.speaker,
            excerpt: synopsis.prose,
            annotations: synopsis.annotations.join(" · "),
            selected: self.selected,
            active,
            zoom,
        };
        rect()
            // Above both nested connection drawing layers.
            .layer(3)
            .position(
                Position::new_absolute()
                    .left(self.node.x * zoom)
                    .top(self.node.y * zoom),
            )
            .width(Size::px(self.node.width * zoom))
            .height(Size::px(HEIGHT * zoom))
            .on_pointer_enter(move |_| {
                if pan.peek().is_none() {
                    hovered.set(Some(enter_id.clone()));
                }
            })
            .on_pointer_leave(move |_| {
                if pan.peek().is_none() {
                    hovered.set(None);
                }
            })
            .on_pointer_down(|event: Event<PointerEventData>| event.stop_propagation())
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .a11y_id(focus)
                    .a11y_focusable(false)
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_alt(format!("Select {title}"))
                    .a11y_builder(move |node| node.set_description(description.clone()))
                    .cursor(CursorIcon::Pointer)
                    .on_press(move |event: Event<PressEventData>| {
                        event.stop_propagation();
                        writer.map_focus.request_focus();
                        let open = opens_script(event.data());
                        writer.selection.set(Some(id.clone()));
                        if open {
                            writer.inspect(&id);
                        }
                    })
                    .child(content),
            )
            .child(
                rect()
                    .position(Position::new_absolute().right(6. * zoom).top(6. * zoom))
                    .layer(1)
                    .child(Handle {
                        positions: self.positions,
                        scene: self.scene.clone(),
                        card: card_key(&self.block),
                        title,
                        x: self.node.x,
                        y: self.node.y,
                        camera: self.camera,
                        visible: active,
                        focusable: self.selected,
                    }),
            )
    }
    fn render_key(&self) -> DiffKey {
        DiffKey::from(&(self.scene.clone(), self.block.id.clone()))
    }
}
fn opens_script(press: &PressEventData) -> bool {
    match press {
        PressEventData::Mouse(mouse) => {
            EventsCombos::<Card>::pressed(mouse.global_location).is_double()
        }
        PressEventData::Touch(touch) => {
            EventsCombos::<Card>::pressed(touch.global_location).is_double()
        }
        PressEventData::Keyboard(_) | PressEventData::Accessibility(_) => true,
    }
}
