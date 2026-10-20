// Copyright 2023 The AccessKit Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0 (found in
// the LICENSE-APACHE file) or the MIT license (found in
// the LICENSE-MIT file), at your option.

use accesskit_consumer::{FilterResult, Node, common_filter_reachable};

#[cfg(test)]
std::thread_local! {
    static FILTER_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
pub(crate) fn take_filter_calls() -> usize {
    FILTER_CALLS.with(|calls| calls.replace(0))
}

/// A node is exposed only when the filtered traversal can reach it from the root.
pub(crate) fn filter(node: &Node) -> FilterResult {
    #[cfg(test)]
    FILTER_CALLS.with(|calls| calls.set(calls.get() + 1));
    common_filter_reachable(node)
}
