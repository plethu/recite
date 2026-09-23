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

The selected beat remains a full manuscript. Line and reply
entries are matched by stable context ID and exact source text. Missing, ambiguous
or changed source matches are not silently rewritten. Use **Refresh from source**
in the catalogue's file actions, or **Source updates** beside a missing entry.
The dedicated screen replaces the scene drawer with a paged list of source excerpts
and scene/beat/speaker metadata. Previous/current source has its changed word region
underlined and bold; nearby extracted source appears in source order, not as an
assertion about the runtime path. Existing translator notes are labelled explicitly;
Recite does not infer author intent. **Read passage** opens the full dialogue.
Back/Forward retains the selected change. **Check source updates** rebuilds a stale
preview. **Update catalogue** applies the complete extraction without requiring
individual inspection or approval. **Review translations** opens the attention queue
afterwards; changed source stays distinct from reviewed translation. Leaving the screen before updating leaves the file untouched. Refresh includes
the current source overlay and saved project scenes. It preserves translations,
translator notes and unknown fields, replaces extracted context/location notes,
marks changed source fuzzy, and keeps removed entries as obsolete records. Named
variants follow their base context; new plural entries use the catalogue's existing
plural rules. Ambiguous contexts and singular/plural shape changes are refused
for explicit migration. Save refuses unsaved translation drafts, a changed source
snapshot, or an externally changed PO file. No gettext executable is needed for
refresh. Conditional groups and destinations retain their
script structure. Source editing remains available in the left column.

Translations are separate in-memory drafts that survive beat/workspace navigation.
Save writes that passage to the actual PO file. Unreviewed drafts carry `fuzzy`;
marking Reviewed clears it only after shared PO validation succeeds. Translation
edits clear pending review. Ctrl/Cmd+S in a translation field saves that translation.
The window refuses to close while PO drafts remain; save or explicitly discard
them first. PO drafts use the shared asynchronous recovery worker. On reopening, recovered
drafts retain the original disk baseline, so external edits still cause conflicts.

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

## Plurals, variants and preview

A plural entry edits all arms together using the catalogue's validated
`Plural-Forms` header. Header names are case-insensitive. Review applies to the
whole entry; invalid placeholders or plural metadata refuse the save without
losing drafts. Named variants have independent drafts and review state. The
entry count control identifies the arm selected by the plural rule.

Trial preview exposes language, count and fallback controls. It uses the shared
runtime and catalogue validation; previewing drafts neither saves nor reviews
them. Source or catalogue changes mark an existing trial stale.

## Limits and checks

Catalogue-scale profiling and automatic file watching remain outstanding.
External PO editors can edit the same file; Compare and Reload handle conflicts
without introducing a private translation database.

Headless interaction tests cover creation, cancellation, invalid language,
existing-path refusal, open/edit/save, plural and variant drafts, review,
external comparison, source refresh and preview locale controls. Shared PO tests
protect unknown data, comments, fuzzy flags and placeholder validation. Physical
keyboard/IME and screen-reader workflows remain in [acceptance](acceptance.md).
