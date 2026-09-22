//! Workspace shortcuts defer to modal owners and skip hidden regions.
use crate::editing::Writer;
use freya::prelude::*;
pub(crate) fn keyboard(
    writer: Writer,
    source: bool,
    editor_id: AccessibilityId,
    event: Event<KeyboardEventData>,
) {
    if *writer.settings_open.peek()
        || writer.localisation.peek().modal_open()
        || writer.command_search.mode.peek().is_some()
    {
        return;
    }
    if let Some(command) = super::shortcut(&event) {
        command.run(writer);
        event.stop_propagation();
        event.prevent_default();
        return;
    }
    if event.modifiers == Modifiers::ALT && matches!(event.code, Code::ArrowLeft | Code::ArrowRight)
    {
        writer.history_step(event.code == Code::ArrowRight);
        event.stop_propagation();
        event.prevent_default();
    } else if event.code == Code::F6 {
        let mut regions = Vec::new();
        if !(*writer.layout.focus.peek()
            || writer.localisation.peek().active
                && writer.localisation.peek().view == crate::localisation::CatalogueView::Updates)
        {
            regions.push(writer.sidebar_focus);
        }
        if writer.localisation.peek().active {
            regions.push(writer.inspector_focus);
        } else if source {
            regions.push(editor_id);
        } else {
            let standalone = writer.layout.standalone();
            let split = writer.layout.show_split(writer);
            if (!standalone && *writer.pane.peek() == crate::editing::Pane::Map) || split {
                regions.push(writer.map_focus);
            }
            if *writer.pane.peek() == crate::editing::Pane::Script {
                regions.push(writer.inspector_focus);
            }
        }
        if regions.is_empty() {
            return;
        }
        let focused = *Platform::get().focused_accessibility_id.peek();
        let index = regions.iter().position(|id| *id == focused).unwrap_or(0);
        let step = if event.modifiers.contains(Modifiers::SHIFT) {
            regions.len() - 1
        } else {
            1
        };
        regions[(index + step) % regions.len()].request_focus();
        event.stop_propagation();
        event.prevent_default();
    } else if crate::editing::is_workspace_key(&event) {
        let mut settings = writer.settings_open;
        settings.set(true);
        event.stop_propagation();
        event.prevent_default();
    } else {
        crate::closing::keyboard(event);
    }
}
