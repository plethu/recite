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

- Physical screen-reader, IME, BiDi, and native window-management acceptance.
- Native macOS and Windows runs; platform shortcut policy is testable on Linux,
  but that does not establish native platform acceptance.
- The pinned Freya CodeEditor preedit presentation defect; see the
  [upstream evidence](upstream-code-editor-preedit.md).
- Full localisation/PO editing, authoring structured conditions and effects,
  preview condition inputs, and scene-manifest-specific build selection.

Script presents existing conditions and effects without executing them.
Preview uses Recite's runtime; game-side effects remain caller-owned requests.
