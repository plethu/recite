# Unix adapter corrections

Source: `accesskit_atspi_common` 0.18.1 from crates.io, upstream revision
`f40dfc01a0c0e76de535969f82fb35e19513737d`, under MIT OR Apache-2.0. The Chromium-derived notice is
retained separately in `LICENSE.chromium`.

The pinned Masonry graph uses AccessKit 0.24.1. Its Unix adapter incorrectly reports disabled
buttons as enabled and sensitive. This copy backports the owning adapter's
[upstream fix](https://github.com/AccessKit/accesskit/commit/6ee0558b6315b3ef1594db24ce45a030ecac7cb5).
The fix is published in `accesskit_atspi_common` 0.21.0, which requires a newer AccessKit family.

The disabled-state patch also removes the redundant `enum_variant_names` suppression in `events.rs`.
No lint baseline or exception is added. Its reproducible source delta is in
[`../patches/accesskit-disabled-state.patch`](../patches/accesskit-disabled-state.patch).

[`../patches/accesskit-filter-exposure.patch`](../patches/accesskit-filter-exposure.patch) fixes
filtered exposure after scrolling. The consumer's clipping filter depends on ancestor transforms,
clip bounds and neighbouring controls, but its change callbacks cover directly updated nodes. The
adapter could therefore advertise controls without registering their D-Bus interfaces. Reconcile the
affected clipping region once per update and announce changed child lists after registration. The
shared filter also checks pruned ancestors through transparent containers, matching initial root
traversal: removing a clipped branch cannot unregister descendants that were never exposed.
Same-bounds text edits retain the direct-node path. The private adapter cost test covers 1,000
controls; `recite-writer-xilem/tests/atspi_exposure.rs` covers scroll reversal, resizing, boundary
siblings, nested clips, hidden parents, focus and event ordering, with exact interface masks across
pruned-branch retirement. The native preferences drill checks real Wayland and D-Bus scrolling. This
is a local correction, not an upstream backport.

Remove this override when the pinned toolkit incorporates both corrections. Run the state and
exposure regressions and the native probe: enabled/sensitive changes, rejection of disabled actions
and complete traversal after scrolling must pass. A successful dispatch alone proves no activation.

The sibling consumer override fixes a separate filtered-iterator defect which can omit a reachable
scrollbar from this adapter's child list. See
[`../accesskit-consumer/UPSTREAM.md`](../accesskit-consumer/UPSTREAM.md) for its bounded source
delta and regression coverage.

[`../patches/accesskit-exposure-cost.patch`](../patches/accesskit-exposure-cost.patch), applied
after the exposure patch, avoids re-filtering a parent whose child-list update is already queued and
uses the sibling consumer's ancestor-aware filter. The parent is marked only after the existing
inclusion checks succeed. A fixed camera-update regression retains all 128 registered controls and
child edges while bounding filter calls at 3,200 (3,112 measured, previously 4,150). These operation
counts establish the local traversal improvement, not native frame latency or GPU performance.

[`../patches/accesskit-exposure-region.patch`](../patches/accesskit-exposure-region.patch), applied
after the exposure-cost patch, filters each reconciled region through one sibling-consumer cache per
tree state. Only the updated node receives a full update. Other region nodes keep their data, so
they are registered or retired only when their exposure changes, and only then are their filtered
parents' child edges diffed. Unchanged nodes therefore receive no notifications, as upstream. Native
Map panning had spent about half its client CPU here. A fixed camera translation of 128 controls now
updates one node with at most two uncached filter calls, previously 3,112, and emits no events. A
translation across the clip registers and retires exactly the direct filter's changed exposure.

[`../patches/accesskit-cached-child-edges.patch`](../patches/accesskit-cached-child-edges.patch),
applied after the region patch, diffs queued child edges through the same per-state caches.
Scrolling a manuscript queues many parents whose children cross the clip; their uncached child
filtering had become most of the remaining adapter cost. The fixed translation across the clip now
makes 36 uncached filter calls, previously 270, all from registering and retiring changed controls.
