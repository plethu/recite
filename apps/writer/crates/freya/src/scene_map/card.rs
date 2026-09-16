//! Beat cards keep dialogue, speaker and structural annotations distinct.
use super::{
    arrangement::Handle,
    camera::Camera,
    layout::{HEIGHT, Node},
    placement::{Arrangement, card_key},
};
use crate::{editing::Writer, palette};
use freya::prelude::*;
use recite_writer_model::{PassageKind, ScriptBlock, ScriptEntry};

// Below this scale, the map becomes a structural overview instead of tiny prose.
pub(super) const DIALOGUE_ZOOM: f32 = 0.6;

#[derive(Clone)]
pub(super) struct Card {
    pub writer: Writer,
    pub block: ScriptBlock,
    pub node: Node,
    pub selected: bool,
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
            && self.ending == other.ending
            && self.scene == other.scene
            && self.writer.dark == other.writer.dark
    }
}
impl Component for Card {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
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
        let active = hovered.read().as_ref() == Some(&id) || focus.is_focused();
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
        let mut content = rect()
            .width(Size::fill())
            .height(Size::fill())
            .padding(10. * zoom)
            .spacing(4. * zoom)
            .background(palette::reading(writer.dark))
            .border(
                Border::new()
                    .width(if self.selected { 2. } else { 1. })
                    .fill(if self.selected {
                        palette::accent(writer.dark)
                    } else if active {
                        palette::muted(writer.dark)
                    } else {
                        palette::rule(writer.dark)
                    }),
            )
            .overflow(Overflow::Clip);
        if zoom >= 0.2 {
            content = content.child(
                label()
                    .text(title.clone())
                    .font_size((18. * zoom).max(14.))
                    .max_lines(1),
            );
        }
        if zoom >= 0.4 {
            content = content.child(
                label()
                    .text(synopsis.speaker)
                    .font_size((11. * zoom).max(11.))
                    .max_lines(1)
                    .color(palette::muted(writer.dark)),
            );
        }
        if zoom >= DIALOGUE_ZOOM {
            content = content.child(
                label()
                    .text(synopsis.prose)
                    .font_family("serif")
                    .font_size((17. * zoom).max(14.))
                    .max_lines(if zoom < 0.8 { 2 } else { 3 }),
            );
        }
        if zoom >= 0.4 && !synopsis.annotations.is_empty() {
            content = content.child(
                label()
                    .text(synopsis.annotations.join(" · "))
                    .font_size((11. * zoom).max(11.))
                    .max_lines(1)
                    .color(palette::muted(writer.dark)),
            );
        }
        rect()
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
                    .a11y_alt(format!("Edit {title}"))
                    .a11y_builder(move |node| node.set_description(description.clone()))
                    .cursor(CursorIcon::Pointer)
                    .on_press(move |event: Event<PressEventData>| {
                        event.stop_propagation();
                        focus.request_focus();
                        writer.inspect(&id);
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
struct Synopsis {
    speaker: String,
    prose: String,
    annotations: Vec<String>,
}
fn synopsis(entries: &[ScriptEntry]) -> Synopsis {
    fn scan(
        entries: &[ScriptEntry],
        effects: &mut usize,
        conditions: &mut usize,
        prose: &mut Option<(String, String)>,
    ) {
        for entry in entries {
            match entry {
                ScriptEntry::Passage(p) if prose.is_none() => {
                    let speaker = match &p.kind {
                        PassageKind::Dialogue { speaker } => speaker
                            .as_deref()
                            .map(palette::display_name)
                            .unwrap_or_else(|| "Dialogue".into()),
                        PassageKind::Choice { .. } => "Reply".into(),
                    };
                    *prose = Some((speaker, p.text.clone()));
                }
                ScriptEntry::Effect(_) => *effects += 1,
                ScriptEntry::Group { entries, .. } => {
                    *conditions += 1;
                    scan(entries, effects, conditions, prose);
                }
                _ => {}
            }
        }
    }
    let (mut effects, mut conditions, mut first) = (0, 0, None);
    scan(entries, &mut effects, &mut conditions, &mut first);
    let (speaker, prose) = first.unwrap_or_else(|| ("".into(), "Continue".into()));
    let mut annotations = Vec::new();
    if conditions > 0 {
        annotations.push("◇ Conditional".into());
    }
    if effects > 0 {
        annotations.push(format!(
            "↗ {effects} effect{}",
            if effects == 1 { "" } else { "s" }
        ));
    }
    Synopsis {
        speaker,
        prose,
        annotations,
    }
}
