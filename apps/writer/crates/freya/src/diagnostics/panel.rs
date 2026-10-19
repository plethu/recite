//! Mount only visible diagnostic buttons; accepted reports stay immutable and shared.
use super::{open, row};
use crate::{
    editing::Writer,
    messages::{MsgId, text},
};
use freya::prelude::*;
use recite_writer_model::DiagnosticSnapshot;

const ROW_HEIGHT: f32 = 48.;
#[derive(Clone)]
pub(crate) struct Panel {
    pub writer: Writer,
    pub editor_id: AccessibilityId,
}
impl PartialEq for Panel {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark && self.editor_id == other.editor_id
    }
}
impl Component for Panel {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let editor_id = self.editor_id;
        let group = use_a11y();
        let mut active = use_state(|| None::<usize>);
        let mut pending = use_state(|| None::<usize>);
        let scroll = use_scroll_controller(ScrollConfig::default);
        crate::design::use_list_reveal(None, active, scroll, ROW_HEIGHT, 0);
        let snapshot = writer
            .buffers
            .model
            .read()
            .as_ref()
            .ok()
            .map(|model| model.document().diagnostic_snapshot());
        let count = snapshot.as_ref().map_or(0, DiagnosticSnapshot::len);
        use_after_side_effect(move || {
            let current_count = writer
                .buffers
                .model
                .read()
                .as_ref()
                .map_or(0, |model| model.document().diagnostic_snapshot().len());
            if active.read().is_some_and(|index| index >= current_count) {
                let next = current_count.checked_sub(1);
                active.set(next);
                pending.set(next);
                if next.is_none() {
                    group.request_unfocus();
                }
            }
        });
        let Some(snapshot) = snapshot.filter(|snapshot| !snapshot.is_empty()) else {
            return rect().into_element();
        };
        let keyboard = snapshot.clone();
        rect()
            .a11y_id(group)
            .a11y_role(AccessibilityRole::Group)
            .a11y_alt(text(MsgId::WriterDiagnostics))
            .a11y_focusable(true)
            .width(Size::fill())
            .height(Size::px(count.min(3) as f32 * ROW_HEIGHT))
            .background(crate::design::palette::current().inset)
            .border(
                Border::new()
                    .width(1.)
                    .fill(crate::design::palette::current().rule),
            )
            .on_key_down(move |event: Event<KeyboardEventData>| {
                navigate(writer, editor_id, &keyboard, active, pending, event);
            })
            .child(
                VirtualScrollView::new_with_data(snapshot, move |item, snapshot| {
                    DiagnosticRow {
                        writer,
                        editor_id,
                        snapshot: snapshot.clone(),
                        index: item.index,
                        active,
                        pending,
                    }
                    .into_element()
                })
                .length(count)
                .item_size(ROW_HEIGHT)
                .scroll_controller(scroll)
                .scroll_with_arrows(false)
                .height(Size::fill())
                .width(Size::fill()),
            )
            .into_element()
    }
}
#[derive(Clone)]
struct DiagnosticRow {
    writer: Writer,
    editor_id: AccessibilityId,
    snapshot: DiagnosticSnapshot,
    index: usize,
    active: State<Option<usize>>,
    pending: State<Option<usize>>,
}
impl PartialEq for DiagnosticRow {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
            && self.editor_id == other.editor_id
            && self.snapshot == other.snapshot
            && self.index == other.index
            && self.active == other.active
            && self.pending == other.pending
    }
}
impl Component for DiagnosticRow {
    fn render_key(&self) -> DiffKey {
        DiffKey::from(&self.index)
    }
    fn render(&self) -> impl IntoElement {
        let id = use_a11y();
        let index = self.index;
        let mut active = self.active;
        let mut pending = self.pending;
        let mut mounted = use_state(|| false);
        let mut dismissed = use_state(|| false);
        let focused = use_focus(id)() == Focus::Keyboard;
        use_after_side_effect(move || {
            if id.is_focused() {
                active.set_if_modified(Some(index));
            } else {
                dismissed.set_if_modified(false);
            }
        });
        use_after_side_effect(move || {
            if !*mounted.read() {
                return;
            }
            if *pending.read() == Some(index) {
                pending.set(None);
                id.request_focus();
            }
        });
        let Some(diagnostic) = self.snapshot.get(index) else {
            return rect().into_element();
        };
        let tooltip = Tooltip::new().child(
            label()
                .width(Size::px(480.))
                .text(diagnostic.message.clone()),
        );
        let button = row(self.writer, self.editor_id, id, diagnostic);
        let writer = self.writer;
        let editor_id = self.editor_id;
        let snapshot = self.snapshot.clone();
        rect()
            .height(Size::px(ROW_HEIGHT))
            .width(Size::fill())
            .on_sized(move |_| mounted.set_if_modified(true))
            .on_key_down(move |event: Event<KeyboardEventData>| {
                if event.key == Key::Named(NamedKey::Escape) {
                    dismissed.set(true);
                    event.stop_propagation();
                    event.prevent_default();
                } else {
                    navigate(writer, editor_id, &snapshot, active, pending, event);
                }
            })
            .child(
                Attached::new(TooltipContainer::new(tooltip.clone()).child(button))
                    .position(AttachedPosition::Top)
                    .maybe_child((focused && !*dismissed.read()).then_some(tooltip)),
            )
            .into_element()
    }
}

// Rows intercept list navigation before the scroll view consumes Home/End.
fn navigate(
    writer: Writer,
    editor_id: AccessibilityId,
    snapshot: &DiagnosticSnapshot,
    mut active: State<Option<usize>>,
    mut pending: State<Option<usize>>,
    event: Event<KeyboardEventData>,
) {
    if !event.modifiers.is_empty() {
        return;
    }
    let count = snapshot.len();
    let current = *active.peek();
    let next = match &event.key {
        Key::Named(NamedKey::Home) => Some(0),
        Key::Named(NamedKey::End) => Some(count - 1),
        Key::Named(NamedKey::ArrowDown) => {
            Some(current.map_or(0, |index| (index + 1).min(count - 1)))
        }
        Key::Named(NamedKey::ArrowUp) => Some(current.unwrap_or(0).saturating_sub(1)),
        Key::Named(NamedKey::PageDown) => {
            Some(current.unwrap_or(0).saturating_add(3).min(count - 1))
        }
        Key::Named(NamedKey::PageUp) => Some(current.unwrap_or(0).saturating_sub(3)),
        Key::Named(NamedKey::Enter) => {
            if let Some(diagnostic) = current.and_then(|index| snapshot.get(index)) {
                open(writer, editor_id, &diagnostic.span);
            }
            event.stop_propagation();
            return;
        }
        _ => return,
    };
    active.set(next);
    pending.set(next);
    event.stop_propagation();
    event.prevent_default();
}
