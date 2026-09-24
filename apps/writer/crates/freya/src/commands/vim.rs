//! Vim navigation owns sequences; writing fields continue to own text editing.
use super::{Command, CommandExt};
mod keyboard;
use crate::{design::tokens as t, editing::Writer};
use freya::prelude::*;
pub(super) use keyboard::{alias, modifier_keyboard};
pub(crate) use keyboard::{keyboard, modifier_key};

#[derive(Clone, Copy)]
pub(crate) struct Navigation {
    pub pending: State<Option<AccessibilityId>>,
    sequence_id: AccessibilityId,
    pub current: State<Option<AccessibilityId>>,
    pub areas: State<Vec<(AccessibilityId, Area)>>,
    pub query: State<String>,
    pub insert_requested: State<bool>,
    pub search_id: AccessibilityId,
    pub matches: State<Vec<String>>,
    pub selected: State<Option<String>>,
}
impl Navigation {
    pub fn new() -> Self {
        let mut pending = use_state(|| None);
        let sequence_id = use_a11y();
        use_after_side_effect(move || {
            if !*Platform::get().is_app_focused.read() {
                pending.set_if_modified(None);
            }
        });
        use_after_side_effect(move || {
            if pending.read().is_some() {
                sequence_id.request_focus();
            }
        });
        let query = use_state(String::new);
        let mut selected = use_state(|| None);
        use_after_side_effect(move || {
            let focused = *Platform::get().focused_accessibility_id.read();
            if pending
                .peek()
                .is_some_and(|id| id != focused && focused != sequence_id)
            {
                pending.set(None);
            }
        });
        use_after_side_effect(move || {
            let _ = query.read();
            selected.set(None);
        });
        Self {
            pending,
            sequence_id,
            current: use_state(|| None),
            areas: use_state(Vec::new),
            query,
            insert_requested: use_state(|| false),
            search_id: use_a11y(),
            matches: use_state(Vec::new),
            selected,
        }
    }
    pub fn owns_keyboard(self) -> bool {
        *Platform::get().focused_accessibility_id.peek() == self.sequence_id
    }
    pub fn area(mut self, id: AccessibilityId, area: Area) {
        if self
            .areas
            .peek()
            .iter()
            .any(|(candidate, previous)| *candidate == id && *previous == area)
        {
            return;
        }
        let mut areas = self.areas.write();
        if let Some(entry) = areas.iter_mut().find(|(candidate, _)| *candidate == id) {
            entry.1 = area;
        } else {
            areas.push((id, area));
        }
    }
    pub fn enter(mut self, id: AccessibilityId) {
        self.current.set_if_modified(Some(id));
    }
    pub fn cancel(mut self) {
        self.pending.set_if_modified(None);
    }
    pub fn step(mut self, writer: Writer, forward: bool) {
        let matches = self.matches.peek();
        if matches.is_empty() {
            return;
        }
        let current = self
            .selected
            .peek()
            .as_ref()
            .and_then(|id| matches.iter().position(|candidate| candidate == id));
        let next = current.map_or(if forward { 0 } else { matches.len() - 1 }, |i| {
            (i + if forward { 1 } else { matches.len() - 1 }) % matches.len()
        });
        let id = matches[next].clone();
        drop(matches);
        writer.inspect(&id);
        if writer.selection.peek().as_deref() == Some(&id) {
            self.selected.set(Some(id));
        }
    }
    pub fn target(self, writer: Writer, dx: f32, dy: f32) -> Option<AccessibilityId> {
        let visible = super::keyboard::regions(
            writer,
            *writer.layout.view.peek() == recite_config::WriterView::Source,
            writer.inspector_focus,
        );
        let areas = self.areas.peek();
        let current = self
            .current
            .peek()
            .filter(|id| visible.contains(id))
            .or_else(|| visible.first().copied())?;
        let from = areas.iter().find(|(id, _)| *id == current)?.1.center();
        areas
            .iter()
            .filter(|(id, _)| *id != current && visible.contains(id))
            .filter_map(|(id, area)| {
                let offset = area.center() - from;
                let forward = offset.x * dx + offset.y * dy;
                (forward > 1. && forward > (offset.x * dy - offset.y * dx).abs())
                    .then_some((*id, offset.x.hypot(offset.y)))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id)
    }
    pub fn focus(self, writer: Writer, dx: f32, dy: f32) {
        if let Some(id) = self.target(writer, dx, dy) {
            id.request_focus();
        }
    }
    pub fn hint(self, writer: Writer) -> Option<Element> {
        self.pending.read().as_ref()?;
        let mut choices = Vec::new();
        for (key, dx, dy, name) in [
            ("h", -1., 0., "left"),
            ("j", 0., 1., "down"),
            ("k", 0., -1., "up"),
            ("l", 1., 0., "right"),
        ] {
            if self.target(writer, dx, dy).is_some() {
                choices.push(format!("{key} {name}"));
            }
        }
        choices.push("Esc cancel".into());
        Some(
            rect()
                .a11y_id(self.sequence_id)
                .a11y_focusable(true)
                .a11y_alt("Pane navigation")
                .on_key_down(move |event| keyboard(writer, &event))
                .position(Position::new_global().bottom(32.).left(24.))
                .layer(Layer::Overlay)
                .padding(t::SPACE_SM)
                .background(t::colors().floating)
                .color(t::colors().ink)
                .border(Border::new().width(1.).fill(t::colors().boundary))
                .child(
                    label()
                        .text(format!("Ctrl+w → {}", choices.join(" · ")))
                        .a11y_role(AccessibilityRole::Status)
                        .a11y_builder(|n| n.set_live(accesskit::Live::Polite)),
                )
                .into_element(),
        )
    }
}
