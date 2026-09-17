# Recite writer

The maintained Freya application for editing Recite dialogue. Scenes open as
maps of beats, branches, convergences, and returns. Select a beat to write beside
the map, or expand independent one-level branch previews to compare replies.
Source uses the same document and compiler diagnostics. Map placement is saved
in personal writer state; pan and zoom leave the dialogue untouched.
Map opens at 100% around the entry beat. Map/Source and presentation preferences
are saved in personal Settings. Fit provides a structural overview;
zooming out hides dialogue and reply labels while keeping beat titles readable.

From the repository root:

```sh
mise exec -- just writer
mise exec -- just writer --project /path/to/project
mise exec -- just check-writer
```

The default opens three original examples: a recurring conversation hub,
a branching and reconverging evacuation, and a three-line conversation.
Example edits live only for the window's lifetime. Project mode provides
explicit save, crash recovery, and external-edit conflict protection.
See the [writer guide](freya-workbench.md).

## Ownership

`crates/authoring` owns source-preserving editing sessions and script projections
from the existing Recite parser, compiler, and runtime. `crates/freya` owns the
native presentation and file-session orchestration. `recite-config` owns user
settings: platform discovery, typed edits, strict validation, comment-preserving
TOML updates, locking, and atomic replacement. The GUI owns only preference
controls and persistence-error presentation. `crates/grammar` binds the
shared tree-sitter grammar for highlighting only.

This workspace has its own lockfile to contain the pinned Freya native graphics
stack. It is maintained application code: the repository verification gate runs
its formatting, tests, and all-target Clippy checks. Other framework candidates
and comparison harnesses have been removed; Git history retains that research.

Native builds need Freya/Skia prerequisites (clang, CMake, pkg-config, and GTK 3
development libraries on Linux) and a working desktop session. Linux execution
and component tests do not establish macOS, Windows, screen-reader, or physical
IME acceptance. See [remaining validation](evidence.md).

Creating a dialogue PO catalogue from the Localise workspace requires GNU
gettext (`msginit` on `PATH`). Install gettext before running the writer's full
test gate, which exercises real catalogue creation. Existing PO editing works
without it. See [the localisation workflow](localisation.md).

The writer uses Freya routing for workspace [history and links](navigation.md),
including Back/Forward across scenes and the translation queue.

Cross-platform installation and packaging remain outstanding; the
[packaging requirements](packaging.md) include desktop deep-link registration,
project activation and delivery to a running window.
