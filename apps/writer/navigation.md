# Workspace navigation

The common header has Back, Forward and Copy link controls. Alt+Left/Right and
mouse Back/Forward buttons use the same history. The history covers Write,
Localise and the translation queue, scenes, beats, passages and Source view.
Queue searches replace the current history entry; each keystroke does not add
another Back step. Returning from a passage restores the search, filter and page.

Navigation commits valid prose drafts through the authoring model. Changing a
saved-project scene requires saving its source edits and applying or discarding
Source drafts first. Switching catalogues requires saving or discarding PO
drafts. A failed navigation restores the history cursor and keeps the current
screen. Temporary example sessions retain their drafts and undo history.

Copy link writes a `recite://writer/...` URL to the clipboard. The routes are
`/write`, `/localise` and `/translations`. Query parameters identify the project,
scene, beat or stable passage ID, catalogue and queue search/filter/page. They
contain locations, not document contents or translation drafts. Catalogue paths
inside the current project are relative to its root.

Open a link on startup, with the corresponding project:

```sh
mise exec -- just writer --project /path/to/project --route 'recite://writer/translations?catalogue=locale/fr.po&q=hello'
```

Links copied from a saved project include its path and can be passed directly as
`recite-writer 'recite://writer/…?project=…'`; the launcher opens that project
before resolving the location. If `--project` is also supplied, both paths must
identify the same project. A catalogue must resolve inside that project.

For the bundled examples, pass `--examples`. Embedded native hosts can provide
`InitialProject` and `InitialRoute(String)` as root contexts. Unavailable scenes, invalid parameters
and links identifying a different project report an error without replacing the
current scene. Freya's router owns the history; the workspace applies its draft
and file checks before accepting the location.

On Linux, another launch forwards its project and link to the existing
project-writer window, including a window opened at the welcome screen. Opening
another project uses the shared loader and refuses to replace unsaved work.
If a clean project opens but its linked location is unavailable, the error
identifies that partial result and the opened project stays selected.
Examples and the component specimen remain independent windows.
Desktop installation acceptance and the other platforms remain
[packaging work](packaging.md), tracked in #79. Dialogs, preview playback and pinned reference snapshots are
transient workspace state and are not encoded in links.

The app shell in `lib.rs` remains the composition owner for shared editor state
and presentation. Route parsing is separate in `navigation/location.rs`;
`navigation.rs` keeps history capture and guarded restoration together because
those operations must agree on the same location. They were reviewed as cohesive
modules; splitting the shell or separating history capture from restoration would
spread the state ownership without simplifying this change.

Source-update review has its own `/source-updates` route. Its `page` parameter
identifies the selected change index within the cached review; queue paging stays
independent. Back/Forward restores that selection while the review remains in memory.
A fresh linked session offers **Check source updates** to build the review from current
project content; link data never authorises a catalogue write. Source-update review
hides the scene drawer temporarily without changing the user's drawer preference.
