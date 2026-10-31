//! Reconcile geometry-dependent filtering without scanning unrelated text updates.
//!
//! A changed transform, clip or child list can change which nodes in the surrounding clip
//! region are exposed. The updated node itself is notified in full. Other region nodes keep
//! their data, so they only change exposure: they are registered or retired, and their
//! filtered parents' child edges are diffed. Platform notifications for unchanged nodes
//! remain limited to nodes the tree reports as updated, as upstream.
use super::*;
use accesskit_consumer::ReachableFilter;

#[derive(Default)]
pub(super) struct ExposureChanges {
    old_subtrees: HashSet<NodeId>,
    new_subtrees: HashSet<NodeId>,
    updated: HashSet<NodeId>,
    parents: HashSet<NodeId>,
    children_events: Vec<(NodeId, ObjectEvent)>,
    /// One update compares a fixed old and new tree state; each is filtered once per node.
    old_filter: ReachableFilter,
    new_filter: ReachableFilter,
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
        self.update_exposed_node(old, new);
        for id in ids {
            let old_node = old.tree_state.node_by_id(id);
            let new_node = new.tree_state.node_by_id(id);
            let old_result = old_node.map(|node| self.exposure.old_filter.filter(&node));
            let new_result = new_node.map(|node| self.exposure.new_filter.filter(&node));
            if old_result == new_result {
                continue;
            }
            match (old_node, new_node) {
                (_, Some(new)) if new_result == Some(FilterResult::Include) => self.add_node(&new),
                (Some(old), _) if old_result == Some(FilterResult::Include) => {
                    self.remove_node(&old)
                }
                _ => {}
            }
            let old_parent = old_node.and_then(|node| {
                node.filtered_parent(&|node| self.exposure.old_filter.filter(node))
            });
            let new_parent = new_node.and_then(|node| {
                node.filtered_parent(&|node| self.exposure.new_filter.filter(node))
            });
            for parent in old_parent.into_iter().chain(new_parent) {
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
        let results = (
            self.exposure.old_filter.filter(old),
            self.exposure.new_filter.filter(new),
        );
        match results {
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
            || self.exposure.old_filter.filter(old) != FilterResult::Include
            || self.exposure.new_filter.filter(new) != FilterResult::Include
            || !self.exposure.parents.insert(new.id())
        {
            return;
        }
        let (old_filter, new_filter) = (&self.exposure.old_filter, &self.exposure.new_filter);
        let events = NodeWrapper(new).children_changes(
            &NodeWrapper(old),
            &|node| old_filter.filter(node),
            &|node| new_filter.filter(node),
        );
        self.exposure
            .children_events
            .extend(events.into_iter().map(|event| (new.id(), event)));
    }

    pub(super) fn emit_children_changed(&mut self) {
        // The backend's callback queue observes registrations before these new edges.
        for (id, event) in self.exposure.children_events.drain(..) {
            self.adapter.emit_object_event(id, event);
        }
    }
}
