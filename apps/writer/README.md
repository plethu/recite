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

This frontend supports project file and beat navigation, manuscript editing, native text undo/redo,
applying changes, checked saves, diagnostics, crash recovery and guarded closing across retained
documents. Switching files preserves field drafts, accepted undo history and recovery ownership.
Save all and Save and close cover every open file. Open in editor saves accepted source while
retaining an unapplied manuscript draft. Reload keeps local work in an independent recovery copy
before accepting disk bytes.

File/selection navigation, disk reload, Apply and applied history prepare in the background. A
pending action can be cancelled while typing continues. Reload checks the captured session before
adoption; a completed recovery archive remains available when cancellation or newer typing prevents
adoption. Save pressed during explicit Apply waits for its result and retirement; a failed or
cancelled Apply leaves the requested save unrun.

The beat outline uses accepted source, including recognizable headers in incomplete documents. Read
beat shows dialogue, replies, conditions, effects and continuations without running them; edit a
passage from that reading list. Add beat, Add line and Add reply use the shared stable-ID editing
operations on workers. Empty documents and empty beats offer creation controls. Leaving a prose
field applies its valid draft before a structural edit or selection change; rejected prose stays
open. Reply and unconditional continuation destinations use the searchable beat picker. Each
structural edit has a separate applied undo step, and Save remains explicit. Duplicate beat names
and unprojectable syntax require external repair; their source is preserved. Long reading lists
mount sixteen entries per page.

Start / restart preview prepares source-language playback off the UI thread, starting at the
selected beat. Continue, choose replies, answer condition requests, and acknowledge simulated
effects; Writer never executes game effects. The compiled run remains fixed and shows a stale notice
after edits. Apply or discard a manuscript draft before restarting. Cancel invalidates publication
while the active compiler phase finishes. Hiding playback retains manuscript typing history.
Catalogue/interpolation preview inputs remain part of the localisation port.

Map and Split share accepted topology, synopsis, personal placement and spatial navigation with
Freya. Camera changes leave source untouched; placement saves beside personal configuration. Native
cards, keyboard selection, move handles, paged Connections and Go to provide equivalent routes
through the scene. Large scenes start with nearby beats; whole-scene Fit uses a structural overview,
and selecting a beat restores a readable native card. Script stays mounted across workspace changes
so its native typing history survives. Geometry and personal placement I/O run on workers; checked
receipts preserve newer typing and reconcile completed writes after cancellation.

Focus writing hides surrounding panes while preserving the mounted manuscript and its native text
history. Exit focus restores the workspace and reveals the previous native focus target. F6 and
Shift+F6 cycle visible panes; in Vim mode, Ctrl+W then h/j/k/l moves by direction and Escape
cancels. Text fields retain ordinary character input. The pane hint and focus changes publish
without waiting for another input event. After accepting a scene/beat lookup, next/previous match
commands (n/N in Vim navigation) repeat that query against current accepted targets. Cancelling a
new lookup preserves the previous search; a failed navigation does not advance it. Saved project
text lookup remains a separate search.

Personal settings and desktop activation also use the shared owners. Localisation, configured
preview inputs, schema tooling and remaining workspace commands are still in progress. The default
launcher continues to use Freya. See [acceptance](acceptance.md) for evidence limits.

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
