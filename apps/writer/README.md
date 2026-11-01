# Recite writer

The native application for editing Recite dialogue, currently a preview, built on Xilem. Script
opens the selected beat in a reading column, with branch previews for comparing replies. Map shows
beats, branches, convergences and returns; it can stand alone or sit beside Script. Source editing
goes to your preferred IDE, with checked saves and explicit import for draft repair.

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
```

The default opens the project welcome screen. `--examples` opens three original examples: a
recurring conversation hub, a branching and reconverging evacuation, and a three-line conversation.
Example edits live only for the window's lifetime. Project mode provides explicit save, crash
recovery, and external-edit conflict protection. See the [writer guide](guide.md).

## Launch options

```sh
mise exec -- just writer run --project /path/to/project --document dialogue/scene.recite \
    --editor nvim
mise exec -- just writer run --design-system
```

Use your editor's own arguments; `--editor-arg` repeats and the source path is appended without a
shell. Omit it if the editor needs no arguments. Without `--editor`, Writer uses the operating
system's preferred application for the file. For a graphical editor that supports it, pass
`--editor-arg --wait` to request a waiting window.

The component specimen uses the production native controls, fonts and palette. Appearance and scale
changes last only for that window; it opens no project and leaves personal preferences untouched.

Writing, Map, preview, localisation, project tools and settings are described in the
[guide](guide.md). Drafts and native typing history survive navigation, and only explicit Save
writes files.

## Build requirements

This workspace has its own lockfile to contain the pinned native graphics stack. It is maintained
application code: the repository verification gate runs its formatting, tests, and all-target Clippy
checks.

Builds need the platform graphics and window-system libraries, pkg-config and a working desktop
session. Linux execution and component tests do not establish macOS, Windows, screen-reader, or
physical IME acceptance; see [acceptance](acceptance.md).

Creating a dialogue PO catalogue from the Localise workspace requires GNU gettext (`msginit` on
`PATH`). Install gettext before running the writer's full test gate, which exercises real catalogue
creation. Existing PO editing works without it. See [the localisation workflow](localisation.md).

Writer supports workspace [history and links](guide.md#navigation-and-links), including Back/Forward
across scenes and the translation queue.

Flatpak, Nix, and native preview package definitions and artifact checks are available; see
[packaging](packaging.md) for build instructions, desktop-link support and the remaining platform
acceptance work.
