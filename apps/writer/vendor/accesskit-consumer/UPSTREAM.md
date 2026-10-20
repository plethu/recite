# Filtered traversal corrections

Source: `accesskit_consumer` 0.36.0 from crates.io, upstream revision
`f40dfc01a0c0e76de535969f82fb35e19513737d`, under MIT OR Apache-2.0. Upstream source and license
notices are retained.

The filtered sibling iterator carries a stale descent flag from an empty transparent container into
the next pruned subtree. It can then stop before later reachable siblings, including a Portal's
scrollbar. Both directions now assign that flag from the current filter result. This is a local
correction. The defect is also present in the locally inspected 0.38.0 source.

The exact source delta (apply with `git apply --unidiff-zero`) is in
[`../patches/accesskit-filtered-siblings.patch`](../patches/accesskit-filtered-siblings.patch). The
`filtered_siblings` tests cover forward, reverse and mixed traversal. Writer's `atspi_exposure`
regression covers the adapter's child list, indices, focus changes and interface ledger. The owned
native `preview-trace` probes exercise scrolling after manuscript typing.

Remove this override when the pinned toolkit's consumer dependency passes these regressions. No lint
suppression or baseline is added.

The Unix adapter's ancestor-aware exposure wrapper previously called the recursive common filter
again for every ancestor. Camera updates amplified that repeated traversal. The additional
`common_filter_reachable` entry point evaluates each ancestor's local exclusion rule once, retaining
the existing focus, hidden, transparent/graft, nearest included parent and adjacent-sibling rules.
It adds no exposure cache. The original common filter and other callers remain unchanged.

Apply [`../patches/accesskit-reachable-filter.patch`](../patches/accesskit-reachable-filter.patch)
after the sibling patch. Differential tests compare the old wrapper with this entry point through
geometry, focus, hiding and child-order updates. The fixed 32-level case reduces base-rule
evaluations from 1,157 to 68. The complete consumer suite and Writer's exact interface-ledger
regressions must pass when replacing this local optimization.

Hardware profiling then found repeated inherited-hidden walks inside those base checks. A single
visibility query proves the target and its raw ancestry unhidden; ancestor checks and their
filtered-parent searches reuse that proof. Sibling branches retain inherited checks, and hidden
ancestry retains the original focused-node exceptions. There is no retained cache. Fixed unclipped
chains at depths 16/32/64 now visit 19/35/67 hidden-state ancestors, versus 2,093/13,653/97,957 in
the original wrapper. Test-only accounting records actual inherited visits. Differential cases
include hidden focused parents, active descendants, attached grafts and hidden adjacent branches;
`just writer check` runs the pinned consumer suite alongside the adapter regressions.
