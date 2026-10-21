//! Reveal only the result of a native Tab move, after Masonry handles the key.
use super::*;

#[derive(Default)]
pub(super) enum Reveal {
    #[default]
    Idle,
    Pending {
        from: Option<WidgetId>,
    },
}
impl Reveal {
    pub fn request(&mut self, from: Option<WidgetId>) {
        *self = Self::Pending { from };
    }
    pub fn synchronize(&mut self, root: &mut RenderRoot) {
        let Self::Pending { from } = std::mem::take(self) else {
            return;
        };
        if root
            .focused_widget()
            .is_some_and(|current| Some(current) != from)
        {
            panes::reveal_focused(root);
        }
    }
}
