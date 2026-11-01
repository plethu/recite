//! Resolve queued accessibility actions against the live owned window, without manufacturing IDs.
use masonry_winit::app::{MasonryState, MasonryUserEvent};
use xilem::masonry::{
    accesskit::NodeId,
    core::{Widget, WidgetRef},
};

pub(super) fn accepts(masonry: &mut MasonryState, event: &MasonryUserEvent) -> bool {
    let MasonryUserEvent::AccessKit(_, accesskit_winit::WindowEvent::ActionRequested(request)) =
        event
    else {
        return true;
    };
    if masonry.is_suspended() {
        return false;
    }
    let mut roots = masonry.roots();
    let (Some(root), None) = (roots.next(), roots.next()) else {
        return false;
    };
    // Recite currently owns one native window and its base layer. Extend this bridge with
    // the host if additional native windows/layers are introduced. Pinned core panics on
    // retired action targets, which can arrive after asynchronous controls disappear.
    target_exists(root.get_layer_root(0), request.target_node)
}
fn target_exists(root: WidgetRef<'_, dyn Widget>, target: NodeId) -> bool {
    let mut pending = vec![root];
    while let Some(widget) = pending.pop() {
        if widget.id() == target {
            return true;
        }
        pending.extend(widget.children());
    }
    false
}

#[cfg(test)]
mod tests;
