//! Explicit dismissal returns focus; a selected action leaves focus to its destination.
use super::*;
use crate::attention::{DocumentMenu, Pane};

#[derive(Default)]
pub(super) struct Focus {
    open: bool,
    origin: Option<WidgetId>,
    last: Option<WidgetId>,
}
impl Focus {
    pub fn synchronize(&mut self, root: &mut RenderRoot, context: &Context) {
        if context.picker_open {
            return;
        }
        if context.document_menu.is_open() && !self.open {
            self.origin = root.focused_widget().or(self.last);
            self.open = panes::focus_pane(root, Pane::DocumentActions);
        } else if !context.document_menu.is_open() && self.open {
            if context.document_menu == DocumentMenu::Dismissed
                && self.origin.is_none_or(|id| !panes::reveal(root, id))
            {
                panes::focus_script(root);
            }
            self.origin = None;
            self.open = false;
        }
        if !self.open {
            self.last = root.focused_widget().or(self.last);
        }
    }
}
pub(super) fn escape(
    context: &Context,
    key: &keyboard::Key,
    modifiers: Modifiers,
) -> Option<Action> {
    (context.document_menu.is_open()
        && !context.picker_open
        && *key == keyboard::Key::Named(NamedKey::Escape)
        && modifiers == Modifiers::default())
    .then_some(Action::DismissDocumentMenu)
}
