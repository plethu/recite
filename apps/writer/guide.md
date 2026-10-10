# Writer guide

The native writer uses Freya and the shared Recite authoring kernel. Run from the repository root:

```sh
mise exec -- just writer run
```

Without arguments, Writer opens the project welcome screen. `--project PATH` opens a project;
`--examples` opens three temporary examples:

- **Relay Hub**: recurring topics, nested returns, and schema-checked requests to set a quest flag
  and change disposition.
- **Floodgate Waterfall**: three choice points, two reconvergences, and two endings.
- **Last Tram Linear**: exactly three spoken sentences with no choices.

Switch scenes in the sidebar. Applied edits, field drafts, undo history, and preview state stay with
each example for the lifetime of the window; nothing is saved to disk. Closing the window discards
these temporary sessions.

Script, Map and Source are workspace views. Script is the first-use default; existing Map and Source
preferences remain valid. Each open scene retains its view and selection within the window. Script
shows the selected beat, with an optional **Script + Map** split. **Focus writing** hides the
surrounding controls until **Exit focus writing** restores them.

In Map, click a card to select it; use **Open script**, Enter or a double click to open its writing
pane; the **Close beat editor** icon or Escape returns to the map. Select the title to rename the
beat in place; Enter applies and Escape cancels. Renaming uses the shared undo history and updates
its references. The sidebar opens each scene as an accordion, with its beats indented underneath in
source order. Selecting a beat opens it in Script and highlights it in Map. Start/end labels
identify entry and exit points; the graph shows branching and convergence rather than imposing a
false parent-child hierarchy on beats.

Expand reply destinations independently to compare branches inside the writing pane. These previews
stay one level deep; **Edit [beat]** enters that beat. Previews show authored content without
executing conditions or effects.

Hover a card or select it with the keyboard to reveal its move grip. Drag the grip, or Tab to it and
use arrow keys. Drag empty map space or scroll to pan. Ctrl+scroll zooms around the pointer (or the
viewport centre when disabled in Settings). Zoom controls include **100%** and **Fit**; fitting does
not rearrange cards. **Arrange automatically** resets card placement while preserving the camera's
zoom and position.

Drag the divider beside the scene drawer or script pane to resize it. Focus a divider and use
Left/Right to resize, or Home/End to reach its bounds. Settings includes **Script pane:
Left/Right**; this preference and the divider widths are restored next launch.

Settings also controls reading size, Source size and UI scale independently. **Commands**
(Ctrl/Cmd+Shift+P) searches workspace actions; **Go to scene or beat** (Ctrl/Cmd+P) searches scene
names and beat IDs. Search cancellation returns focus to the invoking control. Source completion
keeps typing in the editor: Ctrl+Space opens candidates, arrows select, Enter accepts a selected
result, and Escape dismisses. Tab keeps its ordinary editor behaviour.

Map placement is saved with personal preferences and never changes source order or conversation
flow. Corrupt or future layout files are left untouched and reported. Renaming a beat preserves its
placement through its first stable passage ID; effects-only beats use their block name.

Hovering or focusing a beat reveals its connections and reply labels. Selection keeps a persistent
outline. Return routes are identified from dialogue flow, independently of card positions.

**Add beat** creates a separate beat and opens it for writing. **Rename** uses the compiler's rename
operation, preserving passage IDs and updating references. Inline renames refuse cross-scene edits.
Source view also provides a reviewed multi-file rename that lists affected documents before applying
the change. **Add line** inserts before the beat's branching or continuation; **Add reply** turns an
existing unconditional continuation into a reply with the same destination. **Change destination…**
rewires a reply or an unconditional continuation. These changes share the document's undo history.

Prose remains a text field throughout: clicking places the caret without changing the text's layout.
Hover and focus indicate editability without changing text geometry. Leaving a prose field or
changing context applies a source-preserving edit to the working document, with undo available.
Rejected text stays in its field with an explanation and a Discard draft action. Source changes
still use explicit Apply / Discard actions. Script and Source share that working document; neither
saves it automatically.

To edit files in a project:

```sh
mise exec -- just writer run --project /path/to/project
```

The project argument opens that project through Recite's shared manifest discovery. Use **Project**
to reveal the path, open another project, or refresh its context. Select a scene in the sidebar,
edit in Script or Source, and click **Save** or press Ctrl+S (Cmd+S on macOS). Save applies the
current field draft first; rejected prose remains available to repair. Opening another project or
file refuses while the current document has unsaved changes.

Use the sidebar icon beside the Recite name to reclaim sidebar space. Select a beat in the map or
outline to navigate the scene. **Passage actions ▾** opens the selected passage's details and Add
choice menu. Arrow keys move between actions; Escape or Tab returns to the trigger. Source draft
actions stay beside the source editor.

## Navigation and links

The common header has Back, Forward and Copy link controls. Alt+Left/Right and mouse Back/Forward
buttons use the same history. The history covers Write, Localise and the translation queue, scenes,
beats, passages and Source view. Queue searches replace the current history entry; each keystroke
does not add another Back step. Returning from a passage restores the search, filter and page.

Navigation commits valid prose drafts through the authoring model. Changing a saved-project scene
requires saving its source edits and applying or discarding Source drafts first. Switching
catalogues requires saving or discarding PO drafts. A failed navigation restores the history cursor
and keeps the current screen. Temporary example sessions retain their drafts and undo history.

Copy link writes a `recite://writer/...` URL to the clipboard. The routes are `/write`, `/localise`
and `/translations`. Query parameters identify the project, scene, beat or stable passage ID,
catalogue and queue search/filter/page. They contain locations, not document contents or translation
drafts. Catalogue paths inside the current project are relative to its root.

Open a link on startup, with the corresponding project:

```sh
mise exec -- just writer run --project /path/to/project --route 'recite://writer/translations?catalogue=locale/fr.po&q=hello'
```

Links copied from a saved project include its path and can be passed directly as `recite-writer
'recite://writer/…?project=…'`; the launcher opens that project before resolving the location. If
`--project` is also supplied, both paths must identify the same project. A catalogue must resolve
inside that project.

For the bundled examples, pass `--examples`. Embedded native hosts can provide `InitialProject` and
`InitialRoute(String)` as root contexts. Unavailable scenes, invalid parameters and links
identifying a different project report an error without replacing the current scene. Draft and file
checks apply before navigation is accepted.

On Linux, another launch forwards its project and link to the existing project-writer window,
including a window opened at the welcome screen. Opening another project uses the shared loader and
refuses to replace unsaved work. If a clean project opens but its linked location is unavailable,
the error identifies that partial result and the opened project stays selected. Examples and the
component specimen remain independent windows. Desktop integration and platform limits are described
in [packaging](packaging.md). Dialogs, preview playback and pinned reference snapshots are transient
workspace state and are not encoded in links.

Source-update review has its own `/source-updates` route. Its `page` parameter identifies the
selected change index within the cached review; queue paging stays independent. Back/Forward
restores that selection while the review remains in memory. A fresh linked session offers **Check
source updates** to build the review from current project content; link data never authorises a
catalogue write. Source-update review hides the scene drawer temporarily without changing the user's
drawer preference.

## Keyboard and preferences

F6 cycles navigation, map and the open writing pane; Shift+F6 reverses direction. In the map, arrow
keys select the nearest beat in that direction and bring it into view. Enter opens its editor. Slash
focuses beat search; type a name and press Enter to select it. Escape returns from search or writing
to the map. Connections keeps incoming and outgoing links in separate columns, with each reply
beside its source and destination. Return and condition markers remain visible. Hover or keyboard
focus highlights the matching route and its beats; selecting a row reveals the linked beat.
Conversation endings are labelled without a navigation action. Tab reaches controls and connections.

Settings lives at the bottom of the navigation rail; Ctrl+, (Cmd+, on macOS) opens it too. User
preferences include theme, Map/Source, reduced motion, zoom anchoring, exit confirmation, and the
shared Standard/Vim keymap. Vim adds h/j/k/l for map navigation and i to edit; text fields keep
ordinary typing. This is a navigation keymap, not a Vim text editor emulation.

User changes are saved through `recite-config` to the displayed personal configuration path. Project
settings are a separate tab with a manifest editor and an explicit Apply action. They use the same
parser, discovery rules, schema validation and atomic replacement primitives as other clients. A
changed file on disk is never overwritten by a stale Settings draft.

Native trackpad pinch remains unsupported by this pinned backend: Freya does not forward pinch
events, and its winit version does not emit them on Linux. Ctrl+scroll is available, but is not a
substitute for native gesture support. Keyboard component tests do not establish screen-reader or
hardware acceptance.

Shared controls, dialog layout and motion are described in the
[writer design system](design-system.md).

## Save boundary

Save applies the current valid field draft and writes explicitly. If the file changed externally,
replacement is refused and your work stays open. Each replacement retains the previous bytes in a
hidden `.recite-editor-backup-*` file beside the source; backups are not automatically pruned.

Read-only, symlink and non-regular targets are refused. A failure after replacement can mean the new
bytes are already visible. Retry Save to complete durability synchronization, even when no text has
changed. The persistent `.recite-editor.lock` sidecar need not be removed: its OS lock is released
when the process exits.

The lock coordinates cooperating writers; another program can still race the final content check.
ACL/xattr preservation and crash-injection durability are not accepted on every platform. These
limits are distinct from the tested conflict and atomic-replacement behavior.

## Recovery and closing

The writer keeps an atomic JSON recovery snapshot beside the current file, including applied source,
the original saved bytes, the selected field, and any unaccepted field draft. Opening that file
again restores the session. Source files are changed only by Save. A process crash releases the
recovery lock; another writer cannot own the same file's recovery snapshot at the same time. The
empty `.recite-editor-recovery.lock` file may remain and need not be removed.

PO translation drafts also have a locked recovery snapshot beside their catalogue. Reopen that same
catalogue to restore unsaved translations. Standalone declaration TOML drafts recover when you bind
the same source again; the source association itself is not persisted. Incomplete TOML is retained.
Neither recovery path overwrites externally changed files: the original baseline remains the save
conflict boundary. Background snapshots are coalesced, so a crash can lose the latest edit that has
not reached disk. Explicit save, discard and close wait for recovery cleanup.

Closing with translation drafts lists the affected entries and offers direct navigation, save-all
and discard-all actions. Declaration drafts and running jobs have a separate close dialog with save,
discard, cancellation and navigation controls. Failures leave the app open.

Use Ctrl+Q on Linux/Windows or Cmd+Q on macOS to quit. Escape quits only when no element has focus;
focused controls and dialogs consume it first. Native window close uses the same confirmation. The
checkbox in the ordinary close confirmation stores `[writer].confirm_exit = false` in the user's
Recite configuration (the normal platform path, or `RECITE_CONFIG`). Loading never writes settings.
Set it back to `true` to restore the confirmation.

This preference does not bypass unsaved project protection.

Closing an edited file offers **Keep editing**, **Keep recovery and close**, or **Save and close**.
Save failures leave the window open. Tab and Shift+Tab cycle within the dialog, and Escape returns
focus to the previous control.

If the source changed while the writer was closed, recovered work still uses its original baseline:
Save refuses to overwrite the external edit. **Keep recovery copy and reload disk** retains the full
local session in a named `.recite-recovered-*.json` file before loading the current source. The
message shows that copy's path. These copies are retained until you remove them. To restore a
retained copy, close the writer, preserve any existing `<source>.recite-editor-recovery.json`, and
copy the retained JSON to that path before reopening the source. The original baseline still
protects external edits.

Recovery is written after buffer updates, so an abrupt crash can lose changes that have not reached
that update. A corrupt or unsupported snapshot is left untouched and reported; move it aside for
inspection before retrying. Recovery writes can fail on read-only directories or a full disk. Keep
the window open when that happens, and retry Save after resolving the error. ACL/xattr and
crash-injection acceptance remain outstanding across platforms.

## Project context

Diagnostics use all discovered sources, the current document's applied unsaved overlay, and the
configured generated schema manifest. File locations and codes accompany each diagnostic. **Refresh
project context** rereads project sources and schema while retaining the current source, field
draft, and undo history. It marks an existing preview stale. A failed refresh retains the last
accepted context and reports the error.

Preview compiles that same set of effective sources and schema. Script starts at the selected
passage's block; Source starts at the compiled default block. Cross-file links use the normal Recite
reference rules. Existing previews keep their compiled input until explicitly restarted. Effect
requests are displayed without executing game operations. Conditions still require preview inputs
before traversal can continue. Use the project build and trial controls to choose the scene and
supply condition answers; effects remain explicit requests rather than game operations.

Project localisation and catalogue editing are covered in the [localisation guide](localisation.md).
Apply field drafts before they contribute to diagnostics or preview. Native accessibility/platform
requirements remain in [acceptance](acceptance.md).

## Writing trial

The [acceptance guide](acceptance.md) owns real writing sessions, recovery/conflict drills and
native keyboard, IME and screen-reader checks. Run `mise exec -- just writer check` for automated
verification.

Pane dividers preview a new width with a guide while dragging, then reflow on release. Escape
cancels the drag. Keyboard resizing remains immediate.

Large beats page their entries without flattening condition groups. Pin reference keeps a labelled
snapshot alongside the script; Previous/Next beat retraces visits within the current scene. Project
search matches complete words in saved passages and their document, beat and speaker context. Save
updates the affected search index; Refresh discovers new files and refreshes the full saved context.

Initial project open runs in the background. Cancellation reaches compiler and search checkpoints;
an individual filesystem operation or parse finishes before its next checkpoint. Recovery writes are
coalesced on a separate worker; Save and Keep recovery and close still wait for durability. The
[profiling guide](../../docs/profiling-and-optimisation.md#writer-workloads) owns workloads;
[acceptance](acceptance.md#scale-and-performance) owns native scale requirements.
