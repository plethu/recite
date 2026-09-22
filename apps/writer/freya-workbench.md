# Freya file-backed writer

The native writer uses Freya and the shared Recite authoring kernel. Run from the repository root:

```sh
mise exec -- cargo run --locked --manifest-path apps/writer/Cargo.toml -p recite-writer
```

Without arguments, the writer opens three original temporary examples:

- **Relay Hub**: recurring topics, nested returns, and schema-checked requests
  to set a quest flag and change disposition.
- **Floodgate Waterfall**: three choice points, two reconvergences, and two endings.
- **Last Tram Linear**: exactly three spoken sentences with no choices.

Switch scenes in the sidebar. Applied edits, field drafts, undo history, and
preview state stay with each example for the lifetime of the window; nothing
is saved to disk. Closing the window discards these temporary sessions.

Script, Map and Source are workspace views. Script is the first-use default;
existing Map and Source preferences remain valid. Each open scene retains its
view and selection within the window. Script shows the selected beat, with an
optional **Script + Map** split. **Focus writing** hides the surrounding controls
until **Exit focus writing** restores them.

In Map, click a card to select it; use **Open script**, Enter or a double click
to open its writing pane; the **Close beat editor** icon or Escape returns to the map.
Select the title to rename the beat in place; Enter applies and Escape cancels.
Renaming uses the shared undo history and updates its references.
The sidebar opens each scene as an accordion, with its beats indented underneath
in source order. Selecting a beat opens it in Script and highlights it in Map.
Start/end labels identify entry and exit points; the graph shows branching and
convergence rather than imposing a false parent-child hierarchy on beats.

Expand reply destinations independently to compare branches inside the writing
pane. These previews stay one level deep; **Edit [beat]** enters that beat.
Previews show authored content without executing conditions or effects.

Hover a card or select it with the keyboard to reveal its move grip. Drag the
grip, or Tab to it and use arrow keys. Drag empty map space or scroll to pan.
Ctrl+scroll zooms around the pointer (or the viewport centre when disabled in
Settings). Zoom controls include **100%** and **Fit**; fitting
does not rearrange cards. **Arrange automatically** resets card placement while
preserving the camera's zoom and position.

Drag the divider beside the scene drawer or script pane to resize it. The
drawer allows 180–360 logical pixels and the script pane 320–800, constrained
by the space needed for the graph. Focus a divider and use Left/Right to resize,
or Home/End to reach its bounds. Settings includes **Script pane: Left/Right**;
this preference and the divider widths are restored next launch.

Settings also controls reading size, Source size and UI scale independently.
**Commands** (Ctrl/Cmd+Shift+P) searches workspace actions; **Go to scene or beat**
(Ctrl/Cmd+P) searches scene names and beat IDs. Search cancellation returns focus
to the invoking control. Source completion keeps typing in the editor: Ctrl+Space
opens candidates, arrows select, Enter accepts a selected result, and Escape
dismisses. Tab keeps its ordinary editor behaviour.

Card positions survive reopening in `writer-layouts.json`, beside the resolved
user configuration. Project scenes are identified by their full discovered file
path, and examples have a separate namespace. Placement never changes source
order or conversation flow. The writer owns the versioned layout schema;
`recite-config` supplies the same locking and atomic replacement primitive used
by preference edits. Corrupt or future layout files are left untouched and an
error stays visible. Renaming a beat preserves placement through its first
stable passage ID; effects-only beats use their block name.

Return styling comes from deterministic depth-first traversal of the scene,
starting at its declared entry, rather than from card coordinates. Return routes
use separate lanes and attachment points. Hovering a beat isolates its incident
connections while dimming unrelated ones; keyboard focus reveals the same
connections. Selection keeps a persistent outline. Reply labels appear beside
the outgoing connections of the inspected beat. Cards separate speaker and
dialogue from condition/effect annotations. Path emphasis eases over 160 ms,
including when the pointer changes targets mid-transition.

**Add beat** creates a separate beat and opens it for writing. **Rename** uses the
compiler's rename operation, preserving passage IDs and updating references.
Renames that require edits in another scene are refused. **Add line** inserts
before the beat's branching or continuation; **Add reply** turns an existing
unconditional continuation into a reply with the same destination. **Change
destination…** rewires a reply or an unconditional continuation. These changes
share the document's undo history.

Prose remains a text field throughout: clicking places the caret without changing
the text's layout. Hover outlines indicate editability; focus colour settles over
160 ms, without changing text geometry. Leaving a prose field or
changing context applies a source-preserving edit to the working document, with
undo available. Rejected text stays in its field with an explanation and a
Discard draft action. Source changes still use explicit Apply / Discard actions.
Script and Source share that working document; neither saves it automatically.

To edit files in a project:

```sh
cargo run --locked --manifest-path apps/writer/Cargo.toml -p recite-writer -- --project /path/to/project
```

The project argument opens that project through Recite's shared manifest
discovery. Use **Project** to reveal the path, open another project, or refresh
its context. Select a scene in the sidebar,
edit in Script or Source, and click **Save** or press Ctrl+S (Cmd+S on macOS). Save applies the current field
draft first; rejected prose remains available to repair. Opening another project
or file refuses while the current document has unsaved changes.

Use the sidebar icon beside the Recite name to reclaim sidebar space. Select a
beat in the map or outline to navigate the scene. **Passage actions ▾** opens the selected passage's details and
Add choice menu. Arrow keys move between actions; Escape or Tab returns to the
trigger. Source draft actions stay beside the source editor.

## Keyboard and preferences

F6 cycles navigation, map and the open writing pane; Shift+F6 reverses direction.
In the map, arrow keys select the nearest beat in that direction and bring it
into view. Enter opens its editor. Slash focuses beat search; type a name and
press Enter to select it. Escape returns from search or writing to the map.
Connections keeps incoming and outgoing links in separate columns, with each
reply beside its source and destination. Return and condition markers remain
visible. Hover or keyboard focus highlights the matching route and its beats;
selecting a row reveals the linked beat. Conversation endings
are labelled without a navigation action. Tab reaches controls and connections.

Settings lives at the bottom of the navigation rail; Ctrl+, (Cmd+, on macOS)
opens it too. User preferences include theme, Map/Source, reduced motion,
zoom anchoring, exit confirmation, and the shared Standard/Vim keymap. Vim adds
h/j/k/l for map navigation and i to edit; text fields keep ordinary typing.
This is a navigation keymap, not a Vim text editor emulation.

User changes are saved through `recite-config` to the displayed personal
configuration path. Project settings are a separate tab with a manifest editor
and an explicit Apply action. They use the same parser, discovery rules, schema
validation and atomic replacement primitives as other clients. A changed file
on disk is never overwritten by a stale Settings draft.

Native trackpad pinch remains unsupported by this pinned backend: Freya does
not forward pinch events, and its winit version does not emit them on Linux.
Ctrl+scroll is available, but is not a substitute for native gesture support.
Keyboard component tests do not establish screen-reader or hardware acceptance.

Shared controls, dialog layout and motion are described in the
[writer design system](design-system.md).

## Save boundary

The editor compares the file's current bytes with the version it opened or
last saved. A conflict refuses replacement and retains the in-memory work.
Saves use an exclusive cooperative sidecar lock, a synced temporary file in
the same directory and atomic replacement. Each replacement keeps the previous
bytes in a hidden `.recite-editor-backup-*` file beside the source.

Existing permissions are preserved. Symlink/non-regular save targets and
read-only files are refused. On Unix the parent directory is synced, including
on a no-op retry after a failed directory sync. Failure after replacement can
mean the new bytes are already visible; retry Save to complete synchronization.

The lock coordinates these editor instances, not arbitrary third-party writers.
A non-cooperating writer can still race the final content check. The persistent `<source>.recite-editor.lock` sidecar uses an OS-held lock; a
process exit or crash releases that lock. The empty sidecar need not be removed. Backups are retained for inspection and are not
automatically pruned. ACLs/xattrs and crash-injection durability have not been
accepted on every platform.

## Recovery and closing

The writer keeps an atomic JSON recovery snapshot beside the current file,
including applied source, the original saved bytes, the selected field, and any
unaccepted field draft. Opening that file again restores the session. Source
files are changed only by Save. A process crash releases the recovery lock;
another writer cannot own the same file's recovery snapshot at the same time.
The empty `.recite-editor-recovery.lock` file may remain and need not be removed.

PO translation drafts also have a locked recovery snapshot beside their catalogue.
Reopen that same catalogue to restore unsaved translations. Standalone declaration
TOML drafts recover when you bind the same source again; the source association
itself is not persisted. Incomplete TOML is retained. Neither recovery path
overwrites externally changed files: the original baseline remains the save
conflict boundary. Background snapshots are coalesced, so a crash can lose the
latest edit that has not reached disk. Explicit save, discard and close wait
for recovery cleanup.

Closing with translation drafts lists the affected entries and offers direct
navigation, save-all and discard-all actions. Declaration drafts and running
jobs have a separate close dialog with save, discard, cancellation and navigation
controls. Failures leave the app open.

Use Ctrl+Q on Linux/Windows or Cmd+Q on macOS to quit. Escape quits only when
no element has focus; focused controls and dialogs consume it first. Native
window close uses the same confirmation. The checkbox in the ordinary close
confirmation stores `[writer].confirm_exit = false` in the user's Recite
configuration (the normal platform path, or `RECITE_CONFIG`). Loading never
writes settings. Set it back to `true` to restore the confirmation.

This preference does not bypass unsaved project protection.

Closing an edited file offers **Keep editing**, **Keep recovery and close**, or
**Save and close**. Save failures leave the window open. Tab and Shift+Tab cycle
within the dialog, and Escape returns focus to the previous control.

If the source changed while the writer was closed, recovered work still uses
its original baseline: Save refuses to overwrite the external edit. **Keep
recovery copy and reload disk** retains the full local session in a named
`.recite-recovered-*.json` file before loading the current source. The message
shows that copy's path. These copies are retained until you remove them.
To restore a retained copy, close the writer, preserve any existing
`<source>.recite-editor-recovery.json`, and copy the retained JSON to that path
before reopening the source. The original baseline still protects external edits.

Recovery is written after buffer updates, so an abrupt crash can lose changes
that have not reached that update. A corrupt or unsupported snapshot is left
untouched and reported; move it aside for inspection before retrying. Recovery
writes can fail on read-only directories or a full disk. Keep the window open
when that happens, and retry Save after resolving the error. ACL/xattr and
crash-injection acceptance remain outstanding across platforms.

## Project context

Diagnostics use all discovered sources, the current document's applied unsaved
overlay, and the configured generated schema manifest. File locations and
codes accompany each diagnostic. **Refresh project context** rereads project
sources and schema while retaining the current source, field draft, and undo
history. It marks an existing preview stale. A failed refresh retains the last
accepted context and reports the error.

Preview compiles that same set of effective sources and schema. Script starts
at the selected passage's block; Source starts at the compiled default block.
Cross-file links use the normal Recite reference rules. Existing previews keep
their compiled input until explicitly restarted. Effect requests are displayed without executing game operations. Conditions
still require preview inputs before traversal can continue. Scene-manifest-specific
build outputs and runtime fixture inputs remain to be connected.

Project localisation, catalogue editing, completion, and the complete
accessibility/platform requirements remain outstanding. Field drafts must be
applied before their content contributes to diagnostics or preview.

## Writing trial

Use a copy of a real project. Edit in Script and Source, leave a field draft,
close with recovery, and reopen. Then change the source in another editor and
exercise the conflict/reload path. Check keyboard save, dialog navigation,
focus restoration, and whether the recovered field matches what you left.

Component and filesystem tests cover these transitions. Physical IME/BiDi,
screen-reader operation, native window-manager closing, and macOS/Windows
acceptance still need hands-on checks. The pinned CodeEditor preedit limitation
remains recorded in the [text-input evidence](evidence.md).

Focused verification for this isolated workspace:

```sh
mise exec -- cargo test --locked --manifest-path apps/writer/Cargo.toml -p recite-writer-model -p recite-writer
mise exec -- cargo clippy --locked --manifest-path apps/writer/Cargo.toml -p recite-writer-model -p recite-writer --all-targets -- -D warnings
mise exec -- cargo fmt --manifest-path apps/writer/Cargo.toml --all -- --check
scripts/check-test-organization.sh
scripts/check-git-policy.sh
```

Pane dividers preview a new width with a guide while dragging, then reflow on
release. Escape cancels the drag. Keyboard resizing remains immediate.

Large beats page their entries without flattening condition groups. Pin reference
keeps a labelled snapshot alongside the script; Previous/Next beat retraces visits
within the current scene. Project search matches complete words in saved passages
and their document, beat and speaker context. Save updates the affected search
index; Refresh discovers new files and refreshes the full saved context.

Initial project open runs in the background. Cancel opening discards its result;
it does not abort the compiler. Recovery writes are coalesced on a separate worker;
Save and Keep recovery and close still wait for durability. The
[scalability report](scalability.md) documents workloads and current limits.
