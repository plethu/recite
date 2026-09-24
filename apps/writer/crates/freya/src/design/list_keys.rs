//! Stateful list navigation is local to the focused list, never to text insertion.
use super::{keyboard, tokens as t};
use freya::prelude::*;
#[derive(Clone, Copy)]
pub(crate) struct ListKeys(State<Option<AccessibilityId>>);
impl ListKeys {
    pub fn new() -> Self {
        let mut pending = use_state(|| None);
        use_after_side_effect(move || {
            let focus = *Platform::get().focused_accessibility_id.read();
            if pending.peek().is_some_and(|id| id != focus) {
                pending.set(None);
            }
        });
        Self(pending)
    }
    pub fn pending(self) -> bool {
        self.0.read().is_some()
    }
    pub fn cancel(mut self) {
        self.0.set_if_modified(None);
    }
    /// Some(None) consumes the first g without changing the selection.
    pub fn navigate(
        mut self,
        event: &KeyboardEventData,
        normal: bool,
        current: Option<usize>,
        count: usize,
    ) -> Option<Option<usize>> {
        if matches!(
            event.key,
            Key::Named(NamedKey::Shift | NamedKey::Control | NamedKey::Alt | NamedKey::Meta)
        ) {
            return None;
        }
        let pending = self.0.peek().is_some();
        self.0.set_if_modified(None);
        if normal
            && event.modifiers.is_empty()
            && matches!(&event.key, Key::Character(k) if k == "g")
        {
            if pending {
                return Some((count > 0).then_some(0));
            }
            self.0
                .set(Some(*Platform::get().focused_accessibility_id.peek()));
            return Some(None);
        }
        if normal
            && (event.modifiers - Modifiers::SHIFT).is_empty()
            && matches!(&event.key, Key::Character(k) if k == "G")
        {
            return Some(count.checked_sub(1));
        }
        keyboard::list_step(event, normal).map(|step| {
            if count == 0 {
                None
            } else {
                Some(current.map_or(if step > 0 { 0 } else { count - 1 }, |i| {
                    (i as isize + step).rem_euclid(count as isize) as usize
                }))
            }
        })
    }
    pub fn mode(self, normal: bool) -> Element {
        let text = if self.0.read().is_some() {
            "g → g first · Esc cancel"
        } else if normal {
            "NORMAL"
        } else {
            "INSERT"
        };
        label()
            .text(text)
            .font_family("monospace")
            .font_size(t::small())
            .color(t::colors().muted)
            .a11y_role(AccessibilityRole::Status)
            .a11y_builder(|n| n.set_live(accesskit::Live::Polite))
            .into_element()
    }
}
