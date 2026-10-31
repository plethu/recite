// Copyright 2026 The AccessKit Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0 (found in
// the LICENSE-APACHE file) or the MIT license (found in
// the LICENSE-MIT file), at your option.

use core::cell::RefCell;

use accesskit::{Affine, Rect};
use hashbrown::HashMap;

use super::{FilterResult, Geometry, common_filter_base, geometry_filter};
use crate::node::{Node, NodeId};

#[derive(Default)]
struct Memo {
    hidden: HashMap<NodeId, bool>,
    transform: HashMap<NodeId, Affine>,
    base: HashMap<NodeId, Option<FilterResult>>,
    local: HashMap<NodeId, FilterResult>,
    excluded: HashMap<NodeId, bool>,
}

/// Evaluates [`common_filter_reachable`](super::common_filter_reachable) for many
/// nodes of one tree state.
///
/// Inherited visibility, combined transforms and ancestor exclusion are computed
/// once per node, so filtering a region costs work proportional to its nodes rather
/// than nodes times depth. Use one value per tree state; results are not
/// invalidated when the tree changes.
#[derive(Default)]
pub struct ReachableFilter {
    memo: RefCell<Memo>,
}

impl ReachableFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Equals `common_filter_reachable(node)` for the tree state this cache serves.
    pub fn filter(&self, node: &Node) -> FilterResult {
        if self.excluded(node) {
            FilterResult::ExcludeSubtree
        } else {
            self.local(node)
        }
    }

    /// Equals `node.bounding_box()`, reusing ancestor transforms.
    pub fn bounding_box(&self, node: &Node) -> Option<Rect> {
        node.raw_bounds()
            .map(|rect| self.transform(node).transform_rect_bbox(rect))
    }

    fn cached<T: Copy>(
        &self,
        node: &Node,
        map: fn(&mut Memo) -> &mut HashMap<NodeId, T>,
        compute: impl FnOnce() -> T,
    ) -> T {
        if let Some(value) = map(&mut self.memo.borrow_mut()).get(&node.id()) {
            return *value;
        }
        // Computing may consult ancestors through this cache, so no borrow is held.
        let value = compute();
        map(&mut self.memo.borrow_mut()).insert(node.id(), value);
        value
    }

    fn hidden(&self, node: &Node) -> bool {
        self.cached(
            node,
            |memo| &mut memo.hidden,
            || node.data().is_hidden() || node.parent().is_some_and(|parent| self.hidden(&parent)),
        )
    }

    fn transform(&self, node: &Node) -> Affine {
        self.cached(
            node,
            |memo| &mut memo.transform,
            || {
                node.parent()
                    .map_or(Affine::IDENTITY, |parent| self.transform(&parent))
                    * node.direct_transform()
            },
        )
    }

    /// The local rule without parent checks; a visible target proves its ancestry
    /// unhidden, so every node's own inherited visibility is the same input.
    fn base(&self, node: &Node) -> Option<FilterResult> {
        self.cached(
            node,
            |memo| &mut memo.base,
            || common_filter_base(node, || self.hidden(node)),
        )
    }

    fn base_or_include(&self, node: &Node) -> FilterResult {
        self.base(node).unwrap_or(FilterResult::Include)
    }

    fn local(&self, node: &Node) -> FilterResult {
        self.cached(
            node,
            |memo| &mut memo.local,
            || {
                self.base(node).unwrap_or_else(|| {
                    let geometry = Geometry {
                        parent_filter: &|node: &Node| self.base_or_include(node),
                        sibling_filter: &|node: &Node| self.base_or_include(node),
                        bounding_box: &|node: &Node| self.bounding_box(node),
                    };
                    geometry_filter(node, &geometry)
                })
            },
        )
    }

    fn excluded(&self, node: &Node) -> bool {
        self.cached(
            node,
            |memo| &mut memo.excluded,
            || {
                self.local(node) == FilterResult::ExcludeSubtree
                    || node.parent().is_some_and(|parent| self.excluded(&parent))
            },
        )
    }
}
