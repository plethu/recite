# Writer toolkit fixes

Only Masonry, its core and its winit adapter are retained from Xilem revision
`271a27a6d4a930f7878d404f9014e3c50a3a9b88`. Other Xilem packages use that same Git revision. The
workspace manifest resolves their original relative paths to the pinned repository. Screenshots are
omitted; upstream source and Apache-2.0 notices are retained.

Local patches live in [`../patches`](../patches): bounded text undo/redo, an explicit accessible text
field name, placeholder font scaling on mount and live property changes, shared checkbox/radio
indicator sizing on mount and live property changes, clean exit after accessibility
actions, rejection of unhandled Control/Meta text chords, scrollbar normalization after dynamic content
resizing, and safe display-owned clipboard construction. The clipboard change requires the sibling
Copypasta patch. The host also exposes explicit signal delivery after direct native focus changes,
so redraw and accessibility updates complete without another input event. Explicit AltGraph and
Option text remain supported by the text-input guard. These are
local corrections, not verified upstream backports. No unsafe-code lint exception is added to Masonry.
Remove these overrides when equivalent upstream behavior is verified.

Comparison text additionally uses a small ranged-style extension to the existing Masonry Label.
It applies Parley's byte-range styles through the Label's existing layout cache and native text
accessibility path. UTF-8 and brush boundaries are checked; replacing text clears old ranges.
Recite uses bold and underline to show changed words without a colour dependency. This is confined
to read-only comparisons; it does not add rich editing or pointer selection to Label. Consumer tests
cover complete ordered text, wrapping, style replacement and native emphasis metadata. The patch is
[`label-ranged-styles.patch`](../patches/label-ranged-styles.patch).

Map excerpts additionally use the Label's optional positive line cap. Overflow uses an actual
Parley-shaped ellipsis at a logical cluster boundary; short text keeps its natural height. The
complete original layout remains authoritative for accessible text and text ranges. Exact-width
abbreviation caching leaves ordinary Label caching unchanged, and camera-only transforms do not
reshape text. The bounded search retains only a verified fitting candidate and may conservatively
omit more text with complex shaping. An empty visual result is permitted when even the ellipsis
cannot fit. Apply [`label-line-clamp.patch`](../patches/label-line-clamp.patch) after the ranged-style
patch. Writer excerpt tests cover reflow, cache invalidation, Unicode text retention and unchanged
transform cost; native checks must additionally verify complete AT-SPI `GetText` on a visibly
abbreviated excerpt. These Labels do not add pointer selection to the actionable Map cards.

Masonry's compose pass additionally rebuilds an accessibility node only when that widget's own
transform changes. Upstream marked every descendant of a moved widget, although AccessKit nodes
carry parent-relative transforms and descendants are unchanged; panning the Map therefore resent
every mounted card's subtree each frame. Paint is still requested for the whole subtree. The patch is
[`compose-local-accessibility.patch`](../patches/compose-local-accessibility.patch).
`compose_accessibility` covers the emitted node set and consumer-computed descendant bounds; the
Map panning regression bounds nodes per local pan frame. Platform adapters must derive descendant
geometry from ancestor transforms, as the AT-SPI exposure regressions check.

Every Masonry pass also named each parent widget before recursing into its children, only to use
that name in an unreachable panic message. That split each widget type name on every frame. The name
is now formatted only when the panic fires; see
[`recurse-lazy-name.patch`](../patches/recurse-lazy-name.patch).

Twenty unchanged source files have an explicitly approved lint baseline in
`scripts/lint-suppressions-upstream.json`. The lint gate verifies their complete SHA-256 hashes;
changing any byte invalidates that baseline. Modified and newly added files retain normal policy.
