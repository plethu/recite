//! Reconcile geometry-dependent filtering without scanning unrelated text updates.
use super::*;

#[derive(Default)]
pub(super) struct ExposureChanges {
    old_subtrees: HashSet<NodeId>,
    new_subtrees: HashSet<NodeId>,
    updated: HashSet<NodeId>,
    parents: HashSet<NodeId>,
    children_events: Vec<(NodeId, ObjectEvent)>,
}

#[cfg(test)]
mod tests;

pub(super) fn dependencies_changed(old: &Node, new: &Node) -> bool {
    old.raw_bounds() != new.raw_bounds()
        || old.direct_transform() != new.direct_transform()
        || old.clips_children() != new.clips_children()
        || old.is_hidden() != new.is_hidden()
        || old.role() != new.role()
        || old.is_graft() != new.is_graft()
        || old.parent_id() != new.parent_id()
        || !old.child_ids().eq(new.child_ids())
        || old.is_focused() != new.is_focused()
}

fn affected_region<'a>(node: &Node<'a>) -> Node<'a> {
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if ancestor.clips_children() {
            return ancestor;
        }
        parent = ancestor.parent();
    }
    *node
}

fn collect_region(node: Node, visited: &mut HashSet<NodeId>, ids: &mut Vec<NodeId>) {
    if !visited.insert(node.id()) {
        return;
    }
    ids.push(node.id());
    for child in node.children() {
        collect_region(child, visited, ids);
    }
}

impl AdapterChangeHandler<'_> {
    pub(super) fn reconcile_exposure(&mut self, old: &Node, new: &Node) {
        let mut ids = Vec::new();
        collect_region(
            affected_region(old),
            &mut self.exposure.old_subtrees,
            &mut ids,
        );
        collect_region(
            affected_region(new),
            &mut self.exposure.new_subtrees,
            &mut ids,
        );
        for id in ids {
            let old_node = old.tree_state.node_by_id(id);
            let new_node = new.tree_state.node_by_id(id);
            match (old_node, new_node) {
                (Some(old), Some(new)) => self.update_exposed_node(&old, &new),
                (None, Some(new)) if filter(&new) == FilterResult::Include => self.add_node(&new),
                (Some(old), None) if filter(&old) == FilterResult::Include => {
                    self.remove_node(&old)
                }
                _ => {}
            }
            for parent in old_node
                .and_then(|node| node.filtered_parent(&filter))
                .into_iter()
                .chain(new_node.and_then(|node| node.filtered_parent(&filter)))
            {
                if let (Some(old_parent), Some(new_parent)) = (
                    old.tree_state.node_by_id(parent.id()),
                    new.tree_state.node_by_id(parent.id()),
                ) {
                    self.queue_children_changes(&old_parent, &new_parent);
                }
            }
        }
    }

    pub(super) fn update_exposed_node(&mut self, old: &Node, new: &Node) {
        if !self.exposure.updated.insert(new.id()) {
            return;
        }
        match (filter(old), filter(new)) {
            (FilterResult::Include, FilterResult::Include) => {
                let old_wrapper = NodeWrapper(old);
                let new_wrapper = NodeWrapper(new);
                let old_interfaces = old_wrapper.interfaces();
                let new_interfaces = new_wrapper.interfaces();
                let kept = old_interfaces & new_interfaces;
                self.adapter
                    .unregister_interfaces(new.id(), old_interfaces ^ kept);
                self.adapter
                    .register_interfaces(new.id(), new_interfaces ^ kept);
                let bounds = *self.adapter.context.read_root_window_bounds();
                new_wrapper.notify_changes(&bounds, self.adapter, &old_wrapper);
                self.emit_text_selection_change(Some(old), new);
                if new.is_selected() != old.is_selected() {
                    self.enqueue_selection_changed_if_needed(new);
                }
                self.queue_children_changes(old, new);
            }
            (_, FilterResult::Include) => self.add_node(new),
            (FilterResult::Include, _) => self.remove_node(old),
            _ => {}
        }
    }

    fn queue_children_changes(&mut self, old: &Node, new: &Node) {
        // A transformed region reaches the same parent through many descendants.
        // Its children diff has already been queued, including the exposure checks.
        if self.exposure.parents.contains(&new.id())
            || filter(old) != FilterResult::Include
            || filter(new) != FilterResult::Include
            || !self.exposure.parents.insert(new.id())
        {
            return;
        }
        self.exposure.children_events.extend(
            NodeWrapper(new)
                .children_changes(&NodeWrapper(old))
                .into_iter()
                .map(|event| (new.id(), event)),
        );
    }

    pub(super) fn emit_children_changed(&mut self) {
        // The backend's callback queue observes registrations before these new edges.
        for (id, event) in self.exposure.children_events.drain(..) {
            self.adapter.emit_object_event(id, event);
        }
    }
}
