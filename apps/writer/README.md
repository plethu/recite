# Recite writer

The maintained Freya application for editing Recite dialogue, currently a preview. Script opens the
selected beat in a reading column, with branch previews for comparing replies. Map shows beats,
branches, convergences and returns; it can stand alone or sit beside Script. Source uses the same
document and compiler diagnostics.

Script is the first-use default. Existing Map and Source preferences are kept. Presentation settings
and map placement are personal; pan and zoom leave the dialogue untouched. Commands
(Ctrl/Cmd+Shift+P) searches actions, and Go to scene or beat (Ctrl/Cmd+P) searches the open project
or temporary examples.

From the repository root:

```sh
mise exec -- just writer run
mise exec -- just writer run --project /path/to/project
mise exec -- just writer run --examples
mise exec -- just writer check
mise exec -- just writer accessibility
```

The default opens the project welcome screen. `--examples` opens three original examples: a
recurring conversation hub, a branching and reconverging evacuation, and a three-line conversation.
Example edits live only for the window's lifetime. Project mode provides explicit save, crash
recovery, and external-edit conflict protection. See the [writer guide](guide.md).

## Xilem migration

Xilem is the selected replacement frontend. Its file-backed frontend is available separately:

```sh
mise exec -- just writer xilem --project /path/to/project --document dialogue/scene.recite \
    --editor nvim
```

Use your editor's own arguments; `--editor-arg` repeats and the source path is appended without a
shell. Omit it if the editor needs no arguments. Without `--editor`, external opening is disabled.
For a graphical editor that supports it, pass `--editor-arg --wait` to request a waiting window.

This frontend supports project file and passage navigation, manuscript editing, native text
undo/redo, applying changes, checked saves, diagnostics, crash recovery and guarded closing across
retained documents. Switching files preserves field drafts, accepted undo history and recovery
ownership. Save all and Save and close cover every open file. Open in editor saves accepted source
while retaining an unapplied manuscript draft. Reload keeps local work in an independent recovery
copy before accepting disk bytes. Map, localisation, schema tooling, runtime preview, preferences
and desktop activation remain to be ported. The default launcher still uses Freya. See
[acceptance](acceptance.md) for evidence limits.

## Build requirements

This workspace has its own lockfile to contain the pinned Freya native graphics stack. It is
maintained application code: the repository verification gate runs its formatting, tests, and
all-target Clippy checks.

Native builds need Freya/Skia prerequisites (clang, CMake, pkg-config, and GTK 3 development
libraries on Linux) and a working desktop session. Linux execution and component tests do not
establish macOS, Windows, screen-reader, or physical IME acceptance. See
[remaining validation](acceptance.md).

Creating a dialogue PO catalogue from the Localise workspace requires GNU gettext (`msginit` on
`PATH`). Install gettext before running the writer's full test gate, which exercises real catalogue
creation. Existing PO editing works without it. See [the localisation workflow](localisation.md).

The writer uses Freya routing for workspace [history and links](guide.md#navigation-and-links),
including Back/Forward across scenes and the translation queue.

Flatpak, Nix, and native preview package definitions and artifact checks are available; see
[packaging](packaging.md) for build instructions, desktop-link support and the remaining platform
acceptance work.
