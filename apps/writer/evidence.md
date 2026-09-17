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
- Localisation beyond the implemented singular-entry workflow: catalogue
  refresh after source changes, plural/variant editing, and locale/fallback
  preview; see [the current localisation limits](localisation.md#current-limits).
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
