# Writer validation

Freya is selected. The implementation and focused checks now live in
`apps/writer`; retired framework probes and comparison screenshots are available
in Git history rather than the maintained application tree.

Run `mise exec -- just check-writer` for the maintained writer gate. Tests cover
source-preserving prose edits, stable IDs, script structure and links, runtime
routes, file conflicts, recovery, keyboard input, and
component interactions. The shared user configuration has additional tests in
`crates/recite-config/tests`.

## Remaining acceptance

- Cross-platform writer packaging and installation, including the required
  [desktop deep-link registration and activation checks](packaging.md) in #79.
- Physical screen-reader, IME, BiDi, and native window-management acceptance.
- Native trackpad pinch is not forwarded by the pinned Freya backend. Component
  tests cover scroll-to-pan and Ctrl+scroll zoom over cards, but do not establish
  physical trackpad behaviour.
- Native macOS and Windows runs; platform shortcut policy is testable on Linux,
  but that does not establish native platform acceptance.
- The pinned Freya CodeEditor preedit presentation defect; see the
  [upstream evidence](upstream-code-editor-preedit.md).
- Localisation beyond the implemented singular-entry workflow: plural/variant
  editing and locale/fallback preview; see [the current localisation limits](localisation.md#current-limits).
- Authoring structured conditions and effects, preview condition inputs, and
  scene-manifest-specific build selection.

Script presents existing conditions and effects without executing them.
Preview uses Recite's runtime; game-side effects remain caller-owned requests.

## Large-project work

The [scalability report](scalability.md) records the implemented cache, rendering,
navigation and recovery boundaries, repeatable benchmark commands, and measured
10k/100k/1m-passage results. Incremental validation and profiled heap reductions
now have allocation/work-count regression checks; native performance and sustained
large-project memory still need acceptance evidence. New regressions cover
projection identity/invalidation, compact Unicode undo, indexed saved search,
large graph scope/culling, paged prose with undo, pinned reading history, and
background recovery flush/failure/retry.

## Source-update review

The native refresh screen has interaction coverage for leaving without writing,
Back/Forward selection restoration, catalogue update without translation approval,
and refusal when project content changes after review. A 900 × 650 run and native
screenshot inspection confirmed bounded change-list/comparison scrolling and a
visible Update action. Source context is labelled as source order; notes retain
translator provenance. This does not establish native assistive-technology acceptance.

The refresh model, source-context indexing, highlighted comparison and screen are
separate modules. The touched navigation module remains cohesive at 337 lines:
it owns route snapshots, validated history application and navigation controls;
source-refresh policy and rendering stay outside it.

## Shared picker and dialog controls

The approved language-picker study is implemented through `SearchPicker` and
`DialogAction`. Native 900 × 650 screenshots were inspected in dark mode; the
selected state was also inspected at 1600 × 1100 in light mode. Tests check that
opening/searching leaves dialog geometry unchanged, pointer selection works,
keyboard selection reaches beyond the first eight results, and Tab returns to
the dialog order. Vim insertion, NORMAL j/k, and Escape transitions are covered.
Shared submit tests check disabled state and exactly one invocation from either
an input or a button. Catalogue open/create tests use the platform submit chord.

Maintainability review retained `search_picker.rs` (296 lines): it owns one
picker's focus, transient mode, anchored rendering and virtual result list;
search/indexing and locale validation remain with localisation. Splitting its
local state between input and popup components would require shared coordination
without another consumer. `closing.rs` (252 lines) still owns one window-close
lifecycle; shortcut recognition and primary-action rendering are shared modules.

## Follow-up GUI quality pass

Native interaction coverage includes project search beyond 100 results, empty
results and clear/refocus, scene-search keyboard activation, destination search
with undo, manual path submission, dismiss versus retry, and diagnostic cursor
placement. Shared search tests cover Vim INSERT text, NORMAL j/k and Enter.
Translation-status coverage verifies that pending review remains in the attention
queue until its PO edit is saved. Diagnostic offsets account for UTF-16 editor
positions, including non-BMP characters.

The pass reuses concrete search, path, feedback and menu controls without replacing
workspace layouts. Native OS file-dialog presentation/cancellation and assistive
technology/device proofs still belong to packaging acceptance. Freya's current
code editor exposes cursor placement but no public scroll-to-line API: diagnostic
navigation positions the cursor, but an offscreen source line may still require
manual scrolling. Diagnostics outside the current document are shown without an
Open action. These bounds are not covered by the short-document cursor test.

Acceptance: `mise exec -- just check-writer` passed after this pass (workspace
unit/native tests, all-target/all-feature Clippy, and both 10k allocation gates).
The test-organisation and Git-policy checks also passed. Native OS dialogs were
not opened by the headless suite; the manual path route was exercised instead.
