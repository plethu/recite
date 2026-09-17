# Dialogue localisation in the writer

Choose **Localise** (**Localize** with US English UI settings), then **Start
localisation**. The language dropdown searches English names, native names and
locale codes. Choose a result; typing search text alone does not select it.
Common regional/script variants appear by name, and registered locale codes
such as `fr-CA` can also be searched directly. Unknown language tags are rejected
before extraction or file creation.

Searching for **Cofi** reveals **Welsh (Cofi)**, the sole additional dialect
entry. It creates `locale/cy-x-cofi.po`, with `Language: cy-x-cofi`. When plural
entries are present, it requests Welsh plural metadata from gettext. Translations start empty, for the localiser to author. This
private-use locale remains separate from `cy`; the CLI can fall back to a configured Welsh
catalogue through its usual parent-locale lookup. It does not add a translated
Recite UI locale.

Recite creates `locale/<locale>.po` inside the open project and shows that path
before creation. The filename follows the selected locale; it is not another
field to fill in. Examples use the process working folder. Missing destination
folders are created, but an existing file or symlink is never overwritten. The
new catalogue opens at the same beat, with empty translations.

Creation extracts the whole project's dialogue and localisable schema text.
It includes applied edits in the current scene, even if they are not saved yet,
and reads the other scenes from disk. Apply or discard an unapplied Source view
draft first. Extraction errors leave the destination untouched and source IDs
are never silently assigned or changed. Cancelling preparation writes no PO.

Already working with a translator? **Open PO catalogue** connects their existing
file. To create another catalogue, open the current filename beside the language
and choose **Add language**. Save or discard translation drafts before switching.
Language setup stays out of the manuscript once a catalogue is open.

Creation requires GNU gettext's `msginit` on `PATH`. It runs without translator
prompts; Recite sets the target language explicitly and removes inferred author
metadata. Plural entries use gettext's target-language rules and arm count,
validated by Recite. If the installed gettext lacks that language's plural data,
creation reports it without writing a file; update its data or open a prepared PO.
Singular catalogues do not require plural metadata. Opening and editing existing
PO files do not require gettext. Creating a catalogue does not change runtime
locale or fallback configuration. Source-only drafting still needs no catalogue.

The selected beat remains a full manuscript. Existing singular line and reply
entries are matched by stable context ID and exact source text. Missing, ambiguous
or changed source matches are not silently rewritten: update the catalogue with
your existing POT/PO workflow. Conditional groups and destinations retain their
script structure. Source editing remains available in the left column.

Translations are separate in-memory drafts that survive beat/workspace navigation.
Save writes that passage to the actual PO file. Unreviewed drafts carry `fuzzy`;
marking Reviewed clears it only after shared PO validation succeeds. Translation
edits clear pending review. Ctrl/Cmd+S in a translation field saves that translation.
The window refuses to close while PO drafts remain; save or explicitly discard
them first. Unsaved PO edits currently have no crash-recovery journal.

The catalogue path opens file actions. Reload accepts external changes only when
there are no local drafts. Compare shows source, draft and file text for each
pending passage. Keep my drafts remaps them by stable context/source, checks the
compared file fingerprint, and requires a subsequent Save. Discard drafts and use
file explicitly replaces local drafts. If the file changes again before either
acceptance or save, the operation refuses it. PO comments, unrelated flags and
unknown fields are preserved through the shared lossless editor and atomic writer.

The translation queue is a screen in the workspace, with source text, translation
previews, review status and scene/beat context. Search covers source, translation,
scene/beat and stable IDs; the optional Needs attention filter narrows the list.
Results are paged in groups of 32, and changing the search starts at the first
page. Open a passage to read its full beat. Back restores the queue's search,
filter and page; Forward returns to the passage. Read passage leaves the queue
without selecting a different entry.

Entries navigate within the current scene or across an open project using the
extracted file/block comments. Unresolvable entries remain visible without a
fabricated navigation target. Large beats retain the script's 32-entry paging;
selection from the queue reveals the relevant page. See [navigation](navigation.md)
for workspace history and links.

## Current limits

This implements the accepted bilingual direction for existing singular PO entries.
Existing-catalogue refresh/POT regeneration, plural and variant editing, catalogue-scale
profiling, automatic file watching and launching a preferred external editor from
the app are not implemented. External editors can edit the same PO file and the
writer can compare/reload it; there is no private translation database.

Headless Freya interaction tests cover creation, cancellation, invalid language,
existing-path refusal, opening/editing/saving PO, unsaved-close
protection, external comparison, and returning to writing. Core tests cover fuzzy
flag preservation and review-time placeholder validation. Physical keyboard/IME,
screen-reader use, native visual acceptance and frame pacing still need hands-on
checks. The browser study remains in `docs/design/localisation` for design review.
